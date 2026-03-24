//! Decode L-Connect (or our own) USB capture CSVs to extract raw RGB frame data.
//!
//! Reads tshark-exported CSV files produced by `scripts/analyze-usb-meteor.ps1`,
//! reassembles 4-chunk USB packets into 240-byte RF frames, parses headers,
//! extracts compressed payloads, decompresses with tinyuz, and outputs raw
//! RGB pixel data per frame.
//!
//! Usage:
//!   decode-capture <CSV_FILE> [--rx-type <5|6>] [--summary-only] [--output <file>]

use anyhow::{Context, Result};
use clap::Parser;
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

/// USB protocol constants (must match crates/lianli-devices/src/wireless.rs)
const USB_CMD_SEND_RF: u8 = 0x10;
const RF_SELECT: u8 = 0x12;
const RF_SET_RGB: u8 = 0x20;
const RF_CHUNK_SIZE: usize = 60;
const RF_DATA_SIZE: usize = 240;
const RF_DATA_PAYLOAD_OFFSET: usize = 20;
const RF_DATA_PAYLOAD_LEN: usize = RF_DATA_SIZE - RF_DATA_PAYLOAD_OFFSET; // 220

#[derive(Parser)]
#[command(name = "decode-capture", about = "Decode USB capture CSVs to raw RGB frame data")]
struct Cli {
    /// Path to the target-rows CSV file
    csv_file: PathBuf,

    /// Filter by rx_type (e.g. 5 or 6). If omitted, decode all devices.
    #[arg(long)]
    rx_type: Option<u8>,

    /// Only print transfer summaries, no pixel data
    #[arg(long)]
    summary_only: bool,

    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// A parsed row from the capture CSV.
struct CaptureRow {
    frame_number: u64,
    time_relative: f64,
    device_address: u16,
    endpoint: String,
    data_len: usize,
    cap_data: Vec<u8>,
}

/// A reassembled 240-byte RF frame.
#[allow(dead_code)]
struct RfFrame {
    frame_number: u64,
    time_relative: f64,
    channel: u8,
    rx_type: u8,
    data: [u8; RF_DATA_SIZE],
}

/// Parsed header from an RF_SET_RGB header packet.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RgbTransferHeader {
    rx_type: u8,
    device_mac: [u8; 6],
    master_mac: [u8; 6],
    effect_index: u32,
    compressed_len: u32,
    total_frames: u16,
    led_num: u8,
    interval_ms: u16,
    total_packets: u8,
    first_time: f64,
}

/// A complete decoded transfer (header + decompressed pixel data).
struct DecodedTransfer {
    header: RgbTransferHeader,
    decompressed: Vec<u8>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Parse CSV
    let rows = parse_csv(&cli.csv_file)?;
    eprintln!(
        "Parsed {} rows from {}",
        rows.len(),
        cli.csv_file.display()
    );

    // Filter to TX dongle bulk OUT packets.
    // Auto-detect TX device address: look for the address with the most
    // 64-byte EP 0x01 packets whose first byte is 0x10 (USB_CMD_SEND_RF).
    let tx_device_address = {
        let mut addr_counts: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
        for r in &rows {
            if r.endpoint == "0x01" && r.data_len == 64 && r.cap_data.first() == Some(&0x10) {
                *addr_counts.entry(r.device_address).or_default() += 1;
            }
        }
        addr_counts.into_iter().max_by_key(|&(_, cnt)| cnt).map(|(addr, _)| addr).unwrap_or(15)
    };
    eprintln!("Auto-detected TX device address: {}", tx_device_address);

    let tx_rows: Vec<&CaptureRow> = rows
        .iter()
        .filter(|r| r.device_address == tx_device_address && r.endpoint == "0x01" && r.data_len == 64)
        .collect();
    eprintln!("Filtered to {} TX OUT packets", tx_rows.len());

    // Reassemble 4-chunk groups into RF frames
    let rf_frames = reassemble_rf_frames(&tx_rows, cli.rx_type)?;
    eprintln!("Reassembled {} RF frames", rf_frames.len());

    // Group by (rx_type, effect_index) and decode transfers
    let transfers = decode_transfers(&rf_frames)?;
    eprintln!("Decoded {} transfers", transfers.len());

    // Open output
    let mut out: Box<dyn Write> = match &cli.output {
        Some(path) => Box::new(
            std::fs::File::create(path)
                .with_context(|| format!("Cannot create {}", path.display()))?,
        ),
        None => Box::new(io::stdout().lock()),
    };

    // Print summary to stderr
    for t in &transfers {
        let h = &t.header;
        eprintln!(
            "  rx_type={} mac={} effect_index=0x{:08X} frames={} leds={} interval={}ms compressed={}B decompressed={}B",
            h.rx_type,
            format_mac(&h.device_mac),
            h.effect_index,
            h.total_frames,
            h.led_num,
            h.interval_ms,
            h.compressed_len,
            t.decompressed.len(),
        );
    }

    if cli.summary_only {
        return Ok(());
    }

    // Output CSV header
    writeln!(
        out,
        "rx_type,effect_index,interval_ms,total_frames,led_num,frame_idx,led_idx,r,g,b"
    )?;

    // Output pixel data
    for t in &transfers {
        let h = &t.header;
        let leds = h.led_num as usize;
        let frames = h.total_frames as usize;
        let stride = leds * 3;

        for frame_idx in 0..frames {
            let frame_offset = frame_idx * stride;
            for led_idx in 0..leds {
                let pixel_offset = frame_offset + led_idx * 3;
                if pixel_offset + 2 >= t.decompressed.len() {
                    break;
                }
                let r = t.decompressed[pixel_offset];
                let g = t.decompressed[pixel_offset + 1];
                let b = t.decompressed[pixel_offset + 2];
                writeln!(
                    out,
                    "{},0x{:08X},{},{},{},{},{},{},{},{}",
                    h.rx_type,
                    h.effect_index,
                    h.interval_ms,
                    h.total_frames,
                    h.led_num,
                    frame_idx,
                    led_idx,
                    r,
                    g,
                    b,
                )?;
            }
        }
    }

    Ok(())
}

/// Parse the capture CSV file.
fn parse_csv(path: &PathBuf) -> Result<Vec<CaptureRow>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Cannot open {}", path.display()))?;
    let reader = io::BufReader::new(file);
    let mut rows = Vec::new();

    for (line_idx, line) in reader.lines().enumerate() {
        let line = line?;
        if line_idx == 0 {
            continue; // skip header
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let fields = parse_csv_line(line);
        if fields.len() < 6 {
            continue;
        }

        // Support both 7-column (old script: extra FrameLength col) and
        // 6-column (tshark direct: frame.number, frame.time_relative,
        // usb.device_address, usb.endpoint_address, usb.data_len, usb.capdata)
        let cap_data_hex = if fields.len() >= 7 { &fields[6] } else { &fields[5] };
        let data_len_field = if fields.len() >= 7 { &fields[5] } else { &fields[4] };
        let endpoint_field = if fields.len() >= 7 { &fields[3] } else { &fields[3] };
        let cap_data = if cap_data_hex.is_empty() {
            Vec::new()
        } else {
            hex::decode(cap_data_hex).unwrap_or_default()
        };

        rows.push(CaptureRow {
            frame_number: fields[0].parse().unwrap_or(0),
            time_relative: fields[1].parse().unwrap_or(0.0),
            device_address: fields[2].parse().unwrap_or(0),
            endpoint: endpoint_field.clone(),
            data_len: data_len_field.parse().unwrap_or(0),
            cap_data,
        });
    }

    Ok(rows)
}

/// Parse a quoted CSV line into fields.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == ',' && !in_quotes {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

/// Reassemble 4-chunk USB groups into 240-byte RF frames.
fn reassemble_rf_frames(rows: &[&CaptureRow], rx_type_filter: Option<u8>) -> Result<Vec<RfFrame>> {
    let mut frames = Vec::new();
    let mut i = 0;

    while i < rows.len() {
        let row = rows[i];
        if row.cap_data.len() < 4 {
            i += 1;
            continue;
        }

        // Check if this is the start of a 4-chunk group (chunk_idx == 0)
        if row.cap_data[0] != USB_CMD_SEND_RF || row.cap_data[1] != 0x00 {
            i += 1;
            continue;
        }

        let channel = row.cap_data[2];
        let rx_type = row.cap_data[3];

        // Apply rx_type filter
        if let Some(filter) = rx_type_filter {
            if rx_type != filter {
                i += 1;
                continue;
            }
        }

        // Try to collect all 4 chunks
        if i + 3 >= rows.len() {
            break;
        }

        let mut valid = true;
        let mut rf_data = [0u8; RF_DATA_SIZE];

        for chunk_idx in 0u8..4 {
            let r = rows[i + chunk_idx as usize];
            if r.cap_data.len() < 4
                || r.cap_data[0] != USB_CMD_SEND_RF
                || r.cap_data[1] != chunk_idx
                || r.cap_data[2] != channel
                || r.cap_data[3] != rx_type
            {
                valid = false;
                break;
            }

            let payload_len = (r.cap_data.len() - 4).min(RF_CHUNK_SIZE);
            let dest_offset = chunk_idx as usize * RF_CHUNK_SIZE;
            rf_data[dest_offset..dest_offset + payload_len]
                .copy_from_slice(&r.cap_data[4..4 + payload_len]);
        }

        if valid {
            frames.push(RfFrame {
                frame_number: row.frame_number,
                time_relative: row.time_relative,
                channel,
                rx_type,
                data: rf_data,
            });
            i += 4;
        } else {
            i += 1;
        }
    }

    Ok(frames)
}

/// Group RF frames by (rx_type, effect_index) and decode each transfer.
fn decode_transfers(rf_frames: &[RfFrame]) -> Result<Vec<DecodedTransfer>> {
    // Group: key = (rx_type, effect_index), value = vec of RF frames
    let mut groups: BTreeMap<(u8, u32), Vec<&RfFrame>> = BTreeMap::new();

    for frame in rf_frames {
        // Validate RF_SELECT and RF_SET_RGB
        if frame.data[0] != RF_SELECT || frame.data[1] != RF_SET_RGB {
            continue;
        }

        let effect_index = u32::from_be_bytes([
            frame.data[14],
            frame.data[15],
            frame.data[16],
            frame.data[17],
        ]);

        groups
            .entry((frame.rx_type, effect_index))
            .or_default()
            .push(frame);
    }

    let mut transfers = Vec::new();

    for ((rx_type, effect_index), group_frames) in &groups {
        // Separate header packets (index==0) from data packets (index>=1)
        let mut header: Option<RgbTransferHeader> = None;
        let mut data_packets: BTreeMap<u8, &RfFrame> = BTreeMap::new();

        for frame in group_frames {
            let packet_index = frame.data[18];
            let _total_packets = frame.data[19];

            if packet_index == 0 {
                // Header packet — parse if we haven't yet
                if header.is_none() {
                    let mut device_mac = [0u8; 6];
                    let mut master_mac = [0u8; 6];
                    device_mac.copy_from_slice(&frame.data[2..8]);
                    master_mac.copy_from_slice(&frame.data[8..14]);

                    let compressed_len = u32::from_be_bytes([
                        frame.data[20],
                        frame.data[21],
                        frame.data[22],
                        frame.data[23],
                    ]);
                    let total_frames = u16::from_be_bytes([frame.data[25], frame.data[26]]);
                    let led_num = frame.data[27];
                    let interval_ms = u16::from_be_bytes([frame.data[32], frame.data[33]]);

                    header = Some(RgbTransferHeader {
                        rx_type: *rx_type,
                        device_mac,
                        master_mac,
                        effect_index: *effect_index,
                        compressed_len,
                        total_frames,
                        led_num,
                        interval_ms,
                        total_packets: _total_packets,
                        first_time: frame.time_relative,
                    });
                }
            } else {
                // Data packet — store by index (dedup: first wins)
                data_packets.entry(packet_index).or_insert(frame);
            }
        }

        let Some(header) = header else {
            eprintln!(
                "  WARN: no header found for rx_type={} effect_index=0x{:08X}, skipping",
                rx_type, effect_index
            );
            continue;
        };

        // Reassemble compressed data from data packets in order
        let mut compressed = Vec::new();
        for (_idx, frame) in &data_packets {
            let remaining = header.compressed_len as usize - compressed.len();
            if remaining == 0 {
                break;
            }
            let chunk_len = remaining.min(RF_DATA_PAYLOAD_LEN);
            compressed.extend_from_slice(
                &frame.data[RF_DATA_PAYLOAD_OFFSET..RF_DATA_PAYLOAD_OFFSET + chunk_len],
            );
        }

        if compressed.len() < header.compressed_len as usize {
            eprintln!(
                "  WARN: rx_type={} effect_index=0x{:08X}: got {}B compressed, expected {}B",
                rx_type,
                effect_index,
                compressed.len(),
                header.compressed_len
            );
        }

        // Decompress
        let expected_size =
            header.total_frames as usize * header.led_num as usize * 3;

        match lianli_devices::tinyuz::decompress(&compressed, expected_size) {
            Ok(decompressed) => {
                if decompressed.len() != expected_size {
                    eprintln!(
                        "  WARN: rx_type={} effect_index=0x{:08X}: decompressed {}B, expected {}B",
                        rx_type,
                        effect_index,
                        decompressed.len(),
                        expected_size
                    );
                }
                transfers.push(DecodedTransfer {
                    header: header.clone(),
                    decompressed,
                });
            }
            Err(e) => {
                eprintln!(
                    "  ERROR: rx_type={} effect_index=0x{:08X}: decompression failed: {}",
                    rx_type, effect_index, e
                );
            }
        }
    }

    Ok(transfers)
}

/// Format a 6-byte MAC address as XX:XX:XX:XX:XX:XX.
fn format_mac(mac: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}
