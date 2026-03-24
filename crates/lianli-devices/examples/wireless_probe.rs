use anyhow::{Context, Result};
use lianli_transport::usb::{UsbTransport, USB_TIMEOUT};
use std::time::{Duration, Instant};

const TX_VENDOR: u16 = 0x0416;
const TX_PRODUCT: u16 = 0x8040;
const RX_VENDOR: u16 = 0x0416;
const RX_PRODUCT: u16 = 0x8041;

const GET_MAC_CMD: u8 = 0x11;
const TX_RESET_CMD: &str = "11080000";
const SHORT_READ: Duration = Duration::from_millis(400);
const LONG_READ: Duration = Duration::from_millis(1_000);
const DRAIN_READ: Duration = Duration::from_millis(20);

fn main() -> Result<()> {
    let mut tx = UsbTransport::open(TX_VENDOR, TX_PRODUCT).context("opening TX dongle")?;
    tx.detach_and_configure("TX").context("configuring TX dongle")?;

    let mut rx = UsbTransport::open(RX_VENDOR, RX_PRODUCT).context("opening RX dongle")?;
    rx.detach_and_configure("RX").context("configuring RX dongle")?;

    println!("== Wireless Probe ==");
    let master_channel = discover_master_channel(&tx)?;
    println!("master channel: {master_channel}");

    drain("RX pre-drain", &rx);

    let fixed_rx = [
        ("rx_query_34", "10010434", SHORT_READ),
        ("rx_query_37", "10010437", SHORT_READ),
        ("rx_lcd_mode", "10010430", SHORT_READ),
        ("rx_getdev_raw", "10010000", LONG_READ),
        ("rx_getdev_0410", "10010410", LONG_READ),
    ];
    for (label, hex_cmd, timeout) in fixed_rx {
        probe(label, &rx, &decode_command(hex_cmd), timeout)?;
    }

    println!("\n== RX family scan 0x30..0x37 ==");
    for suffix in 0x30u8..=0x37u8 {
        let mut cmd = vec![0u8; 64];
        cmd[0] = 0x10;
        cmd[1] = 0x01;
        cmd[2] = 0x04;
        cmd[3] = suffix;
        probe(&format!("rx_family_{suffix:02x}"), &rx, &cmd, SHORT_READ)?;
    }

    println!("\n== TX probes ==");
    let fixed_tx = [
        ("tx_getdev_raw", "10010000", LONG_READ),
        ("tx_getdev_0410", "10010410", LONG_READ),
    ];
    for (label, hex_cmd, timeout) in fixed_tx {
        probe(label, &tx, &decode_command(hex_cmd), timeout)?;
    }

    println!("\n== Channel-coded GetDev sweep (RX) ==");
    sweep_template("rx_getdev_b1ch", &rx, SHORT_READ, |channel| {
        let mut cmd = vec![0u8; 64];
        cmd[0] = 0x10;
        cmd[1] = channel;
        cmd
    })?;
    sweep_template("rx_getdev_b2ch", &rx, SHORT_READ, |channel| {
        let mut cmd = vec![0u8; 64];
        cmd[0] = 0x10;
        cmd[1] = 0x01;
        cmd[2] = channel;
        cmd
    })?;
    sweep_template("rx_getdev_b3ch", &rx, SHORT_READ, |channel| {
        let mut cmd = vec![0u8; 64];
        cmd[0] = 0x10;
        cmd[1] = 0x01;
        cmd[3] = channel;
        cmd
    })?;
    sweep_template("rx_getdev_04ch", &rx, SHORT_READ, |channel| {
        let mut cmd = vec![0u8; 64];
        cmd[0] = 0x10;
        cmd[1] = 0x01;
        cmd[2] = 0x04;
        cmd[3] = channel;
        cmd
    })?;

    println!("\n== Channel-coded GetDev sweep (TX) ==");
    sweep_template("tx_getdev_b1ch", &tx, SHORT_READ, |channel| {
        let mut cmd = vec![0u8; 64];
        cmd[0] = 0x10;
        cmd[1] = channel;
        cmd
    })?;
    sweep_template("tx_getdev_b2ch", &tx, SHORT_READ, |channel| {
        let mut cmd = vec![0u8; 64];
        cmd[0] = 0x10;
        cmd[1] = 0x01;
        cmd[2] = channel;
        cmd
    })?;

    println!("\n== Daemon-style sequence after TX reset ==");
    send_only("tx_reset", &tx, &decode_command(TX_RESET_CMD))?;
    std::thread::sleep(Duration::from_millis(50));
    drain("RX after reset", &rx);

    let daemon_rx = [
        ("daemon_rx_34", "10010434", SHORT_READ),
        ("daemon_rx_37", "10010437", SHORT_READ),
        ("daemon_rx_30", "10010430", SHORT_READ),
        ("daemon_getdev", "10010000", LONG_READ),
    ];
    for (label, hex_cmd, timeout) in daemon_rx {
        probe(label, &rx, &decode_command(hex_cmd), timeout)?;
    }

    Ok(())
}

fn discover_master_channel(tx: &UsbTransport) -> Result<u8> {
    let channels_to_try = preferred_channels();

    let mut first = None;

    for channel in channels_to_try {
        let mut cmd = vec![0u8; 64];
        cmd[0] = GET_MAC_CMD;
        cmd[1] = channel;

        tx.write_bulk(&cmd, USB_TIMEOUT)
            .with_context(|| format!("sending GET_MAC on channel {channel}"))?;

        let mut response = [0u8; 64];
        if let Ok(len) = tx.read_bulk(&mut response, LONG_READ) {
            if len >= 7 && response[0] == GET_MAC_CMD && response[1..7].iter().any(|&b| b != 0) {
                println!(
                    "get_mac[{channel:02}] => {:02x?}",
                    &response[..len.min(16)]
                );
                if first.is_none() {
                    first = Some(channel);
                }
            }
        }
    }

    first.context("could not discover master channel")
}

fn preferred_channels() -> Vec<u8> {
    std::iter::once(8u8)
        .chain((2..=38).filter(|&ch| ch != 8 && ch % 2 == 0))
        .chain((1..=39).filter(|&ch| ch % 2 == 1))
        .collect()
}

fn probe(label: &str, transport: &UsbTransport, cmd: &[u8], timeout: Duration) -> Result<()> {
    drain(&format!("{label} pre-drain"), transport);

    let started = Instant::now();
    transport
        .write_bulk(cmd, USB_TIMEOUT)
        .with_context(|| format!("sending {label}"))?;

    let mut response = [0u8; 512];
    match transport.read_bulk(&mut response, timeout) {
        Ok(len) => {
            println!(
                "{label:<18} ok  after {:>4}ms  len={:<3}  hdr={:02x?}",
                started.elapsed().as_millis(),
                len,
                &response[..len.min(16)]
            );
            if label.starts_with("rx_getdev_b1ch[") && label.contains("[08]") {
                dump_getdev_records(label, &response[..len]);
            }
        }
        Err(lianli_transport::TransportError::Usb(rusb::Error::Timeout)) => {
            println!(
                "{label:<18} timeout after {:>4}ms",
                started.elapsed().as_millis()
            );
        }
        Err(err) => {
            println!("{label:<18} error  {err}");
        }
    }

    drain(&format!("{label} post-drain"), transport);
    Ok(())
}

fn sweep_template<F>(
    label_prefix: &str,
    transport: &UsbTransport,
    timeout: Duration,
    build: F,
) -> Result<()>
where
    F: Fn(u8) -> Vec<u8>,
{
    let mut saw_success = false;

    for channel in preferred_channels() {
        let label = format!("{label_prefix}[{channel:02}]");
        let cmd = build(channel);
        let response = probe_capture(&label, transport, &cmd, timeout)?;
        if response.is_some() {
            saw_success = true;
        }
    }

    if !saw_success {
        println!("{label_prefix:<18} no responses on any tested channel");
    }

    Ok(())
}

fn probe_capture(
    label: &str,
    transport: &UsbTransport,
    cmd: &[u8],
    timeout: Duration,
) -> Result<Option<usize>> {
    drain(&format!("{label} pre-drain"), transport);

    let started = Instant::now();
    transport
        .write_bulk(cmd, USB_TIMEOUT)
        .with_context(|| format!("sending {label}"))?;

    let mut response = [0u8; 512];
    let outcome = match transport.read_bulk(&mut response, timeout) {
        Ok(len) => {
            println!(
                "{label:<18} ok  after {:>4}ms  len={:<3}  hdr={:02x?}",
                started.elapsed().as_millis(),
                len,
                &response[..len.min(16)]
            );
            if label.starts_with("rx_getdev_b1ch[") && label.contains("[08]") {
                dump_getdev_records(label, &response[..len]);
            }
            Some(len)
        }
        Err(lianli_transport::TransportError::Usb(rusb::Error::Timeout)) => {
            println!(
                "{label:<18} timeout after {:>4}ms",
                started.elapsed().as_millis()
            );
            None
        }
        Err(err) => {
            println!("{label:<18} error  {err}");
            None
        }
    };

    drain(&format!("{label} post-drain"), transport);
    Ok(outcome)
}

fn send_only(label: &str, transport: &UsbTransport, cmd: &[u8]) -> Result<()> {
    drain(&format!("{label} pre-drain"), transport);
    transport
        .write_bulk(cmd, USB_TIMEOUT)
        .with_context(|| format!("sending {label}"))?;
    println!("{label:<18} sent only");
    Ok(())
}

fn drain(label: &str, transport: &UsbTransport) {
    let mut count = 0usize;
    let mut buf = [0u8; 64];

    loop {
        match transport.read_bulk(&mut buf, DRAIN_READ) {
            Ok(len) => {
                count += 1;
                println!(
                    "{label:<18} stale[{count}] len={:<3} hdr={:02x?}",
                    len,
                    &buf[..len.min(16)]
                );
                if count >= 8 {
                    break;
                }
            }
            Err(lianli_transport::TransportError::Usb(rusb::Error::Timeout)) => break,
            Err(err) => {
                println!("{label:<18} drain error {err}");
                break;
            }
        }
    }
}

fn decode_command(prefix: &str) -> Vec<u8> {
    let mut bytes = hex::decode(prefix).expect("valid probe command");
    bytes.resize(64, 0u8);
    bytes
}

fn dump_getdev_records(label: &str, response: &[u8]) {
    if response.len() < 4 || response[0] != 0x10 {
        return;
    }

    let device_count = response[1] as usize;
    if device_count == 0 || device_count > 12 {
        return;
    }

    println!("{label:<18} decoded device_count={device_count}");

    let mut offset = 4usize;
    for idx in 0..device_count {
        if offset + 42 > response.len() {
            println!("{label:<18} record[{idx}] truncated");
            break;
        }

        let record = &response[offset..offset + 42];
        let mac = &record[0..6];
        let master = &record[6..12];
        let fan_types = &record[24..28];
        let rpms = [
            u16::from_be_bytes([record[28], record[29]]),
            u16::from_be_bytes([record[30], record[31]]),
            u16::from_be_bytes([record[32], record[33]]),
            u16::from_be_bytes([record[34], record[35]]),
        ];
        println!(
            "{label:<18} record[{idx}] mac={:02x?} master={:02x?} ch={} rx={} type=0x{:02x} fan_count={} fan_types={:02x?} rpms={:?} pwm={:02x?} raw={:02x?}",
            mac,
            master,
            record[12],
            record[13],
            record[18],
            record[19],
            fan_types,
            rpms,
            &record[36..40],
            record
        );

        offset += 42;
    }
}
