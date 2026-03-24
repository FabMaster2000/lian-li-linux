use anyhow::{bail, Context, Result};
use lianli_transport::usb::{UsbTransport, USB_TIMEOUT};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::fmt;
use std::sync::{
    atomic::{AtomicBool, AtomicU16, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::{debug, info, warn};

const TX_VENDOR: u16 = 0x0416;
const TX_PRODUCT: u16 = 0x8040;
const RX_VENDOR: u16 = 0x0416;
const RX_PRODUCT: u16 = 0x8041;

const USB_CMD_SEND_RF: u8 = 0x10;
const USB_CMD_GET_MAC: u8 = 0x11;

const RF_SELECT: u8 = 0x12;
const RF_PWM_CMD: u8 = 0x10;
const RF_SET_RGB: u8 = 0x20;

const RF_DATA_SIZE: usize = 240;
const RF_CHUNK_SIZE: usize = 60;
const RF_CHUNKS: usize = RF_DATA_SIZE / RF_CHUNK_SIZE;

static CMD_RESET: Lazy<Vec<u8>> = Lazy::new(|| decode_command("11080000"));
static CMD_VIDEO_START: Lazy<Vec<u8>> = Lazy::new(|| decode_command("11010000"));
static CMD_RX_QUERY_34: Lazy<Vec<u8>> = Lazy::new(|| decode_command("10010434"));
static CMD_RX_QUERY_37: Lazy<Vec<u8>> = Lazy::new(|| decode_command("10010437"));
static CMD_RX_LCD_MODE: Lazy<Vec<u8>> = Lazy::new(|| decode_command("10010430"));

const GET_DEV_TIMEOUT: Duration = Duration::from_millis(1_000);
const RX_DRAIN_TIMEOUT: Duration = Duration::from_millis(10);
const RX_PRIME_SETTLE: Duration = Duration::from_millis(50);
const DISCOVERY_RESCAN_THRESHOLD: u16 = 3;
const DISCOVERY_DEVICE_GRACE_POLLS: u8 = 24;
const FAN_PWM_SEND_REPEATS: u8 = 3;
const FAN_PWM_REPEAT_GAP: Duration = Duration::from_millis(5);

fn decode_command(prefix: &str) -> Vec<u8> {
    let mut bytes = hex::decode(prefix).expect("valid hex literal");
    bytes.resize(64, 0u8);
    bytes
}

fn preferred_scan_channels(seed: u8) -> Vec<u8> {
    let mut channels = Vec::with_capacity(40);
    for channel in std::iter::once(seed)
        .chain(std::iter::once(8u8))
        .chain((2..=38).filter(|&ch| ch % 2 == 0))
        .chain((1..=39).filter(|&ch| ch % 2 == 1))
    {
        if channel == 0 || channels.contains(&channel) {
            continue;
        }
        channels.push(channel);
    }
    channels
}

/// Wireless fan device type, determines minimum duty and RPM curves.
///
/// Byte ranges for classifying fan type:
/// ```text
/// SLV3  (base 20): 20-26  (LED: 20-23, LCD: 24-26)
/// TLV2  (base 27): 27-35  (LCD: 27,32-35, LED: 28-31)
/// SLINF (base 36): 36-39  (LED only)
/// RL120:           40
/// CLV1:            41-42
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WirelessFanType {
    /// SLV3 120mm/140mm LED fans (no LCD) — 14% minimum duty
    Slv3Led,
    /// SLV3 120mm/140mm LCD fans — 14% minimum duty
    Slv3Lcd,
    /// TLV2 120mm/140mm LCD fans — 10% minimum duty
    Tlv2Lcd,
    /// TLV2 120mm/140mm LED fans (no LCD) — 11% minimum duty
    Tlv2Led,
    /// SL-INF wireless fans — 11% minimum duty
    SlInf,
    /// CL / RL120 fans — 10% minimum duty (special PWM filter)
    Clv1,
    /// Unknown fan type
    Unknown,
}

impl WirelessFanType {
    /// Minimum duty percentage for this fan type.
    pub fn min_duty_percent(self) -> u8 {
        match self {
            Self::Slv3Led | Self::Slv3Lcd => 14,
            Self::Tlv2Lcd => 10,
            Self::Tlv2Led | Self::SlInf => 11,
            Self::Clv1 => 10,
            Self::Unknown => 10,
        }
    }

    /// Human-readable display name for this fan type.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Slv3Led => "UNI FAN SL V3 Wireless",
            Self::Slv3Lcd => "UNI FAN SL V3 Wireless LCD",
            Self::Tlv2Lcd => "UNI FAN TL Wireless LCD",
            Self::Tlv2Led => "UNI FAN TL Wireless",
            Self::SlInf => "UNI FAN SL-INF Wireless",
            Self::Clv1 => "UNI FAN CL Wireless",
            Self::Unknown => "Wireless Fan",
        }
    }

    /// Number of addressable LEDs per fan for this device type.
    ///
    /// LED counts per device type:
    /// - TLV2: 104 LEDs per zone (UP/DOWN combined, ~26 per fan)
    /// - SLV3: 160 LEDs per zone (inner + outer rings, ~40 per fan)
    /// - SL-INF: 176 LEDs total across all fans (~44 per fan)
    /// - CL: ~24 LEDs per fan (outer + center)
    pub fn leds_per_fan(self) -> u8 {
        match self {
            Self::Tlv2Lcd | Self::Tlv2Led => 26,
            Self::Slv3Led | Self::Slv3Lcd => 40,
            Self::SlInf => 44,
            Self::Clv1 => 24,
            Self::Unknown => 20,
        }
    }

    /// Whether the receiver firmware supports direct motherboard PWM sync.
    ///
    /// SLV3 receivers have a physical PWM header — sending PWM=[6,6,6,6]
    /// tells the firmware to read from that header instead. Other devices
    /// (TLV2, SL-INF, CL) need the host to poll and relay mobo PWM.
    pub fn supports_hw_mobo_sync(self) -> bool {
        matches!(self, Self::Slv3Led | Self::Slv3Lcd)
    }

    /// Classify fan type from the fan-type byte in the device record.
    ///
    /// Byte ranges for classifying fan type:
    ///   `(num < 27) ? SLV3Fan : (num < 36) ? TLV2Fan : SLINF`
    /// Within SLV3/TLV2, bytes base+4..base+7 have LCD.
    fn from_fan_type_byte(b: u8) -> Self {
        match b {
            20..=23 => Self::Slv3Led,          // SLV3 LED (120/140, normal/reverse)
            24..=26 => Self::Slv3Lcd,          // SLV3 LCD (120/140, normal/reverse)
            27 | 32..=35 => Self::Tlv2Lcd,     // TLV2 LCD
            28..=31 => Self::Tlv2Led,          // TLV2 LED (120/140, normal/reverse)
            36..=39 => Self::SlInf,            // SL-INF (LED only)
            40 => Self::Clv1,                  // RL120
            41..=42 => Self::Clv1,             // CLV1 variants
            _ => Self::Unknown,
        }
    }
}

/// A wireless device discovered via the RX GetDev command.
/// Parsed from the 42-byte device record in the response.
#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    /// Device MAC address (6 bytes)
    pub mac: [u8; 6],
    /// Master MAC this device is bound to (6 bytes)
    pub master_mac: [u8; 6],
    /// RF channel this device communicates on
    pub channel: u8,
    /// RX type (radio endpoint address, unique per device)
    pub rx_type: u8,
    /// Device type byte (0=fan group, 65=LC217 LCD, 255=master)
    pub device_type: u8,
    /// Number of fans connected (0-4)
    pub fan_count: u8,
    /// Fan type bytes for each slot (determines fan model)
    pub fan_types: [u8; 4],
    /// Current fan RPMs (read from device, big-endian u16 x4)
    pub fan_rpms: [u16; 4],
    /// Current PWM values being applied (0-255 x4)
    pub current_pwm: [u8; 4],
    /// Command sequence number
    pub cmd_seq: u8,
    /// Classified fan type for the device
    pub fan_type: WirelessFanType,
    /// Index in the discovery list (used for video mode prep)
    pub list_index: u8,
    /// How many discovery polls in a row this device was missing from.
    pub missed_polls: u8,
}

impl DiscoveredDevice {
    /// MAC address as a colon-separated hex string.
    pub fn mac_str(&self) -> String {
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.mac[0], self.mac[1], self.mac[2],
            self.mac[3], self.mac[4], self.mac[5],
        )
    }
}

impl fmt::Display for DiscoveredDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({:?}, {} fans, ch={}, rx={})",
            self.mac_str(),
            self.fan_type,
            self.fan_count,
            self.channel,
            self.rx_type,
        )
    }
}

#[derive(Debug, Clone)]
struct SimulatedWireless {
    fan_type: WirelessFanType,
    fan_type_byte: u8,
    fan_count: u8,
    mac: [u8; 6],
    master_mac: [u8; 6],
    channel: u8,
    rx_type: u8,
    mobo_pwm: Option<u8>,
}

impl SimulatedWireless {
    fn from_env() -> Option<Self> {
        let raw = std::env::var("LIANLI_SIM_WIRELESS").ok()?;
        let raw = raw.trim().to_string();
        if raw.is_empty() {
            return None;
        }

        let mut parts = raw
            .split(|c| c == ':' || c == ',' || c == ';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());

        let kind_raw = parts.next().unwrap_or_default().to_ascii_lowercase();
        let kind = kind_raw.replace('-', "").replace('_', "");

        let fan_count = parts
            .next()
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(3)
            .clamp(1, 4);

        let (fan_type, fan_type_byte) = if kind.contains("slinf") {
            // 36..=39 are SL-INF; default to 120mm (36). If "140" is present, use 37.
            let byte = if kind.contains("140") { 37 } else { 36 };
            (WirelessFanType::SlInf, byte)
        } else {
            warn!(
                "LIANLI_SIM_WIRELESS='{raw}' not supported (expected slinf[:1-4])"
            );
            return None;
        };

        Some(Self {
            fan_type,
            fan_type_byte,
            fan_count,
            mac: [0x02, 0x11, 0x22, 0x33, 0x44, 0x55],
            master_mac: [0x10, 0x20, 0x30, 0x40, 0x50, 0x60],
            channel: 8,
            rx_type: 1,
            mobo_pwm: None,
        })
    }

    fn build_device(&self) -> DiscoveredDevice {
        let mut fan_types = [0u8; 4];
        for slot in 0..self.fan_count.min(4) as usize {
            fan_types[slot] = self.fan_type_byte;
        }

        DiscoveredDevice {
            mac: self.mac,
            master_mac: self.master_mac,
            channel: self.channel,
            rx_type: self.rx_type,
            device_type: 0,
            fan_count: self.fan_count,
            fan_types,
            fan_rpms: [0, 0, 0, 0],
            current_pwm: [0, 0, 0, 0],
            cmd_seq: 1,
            fan_type: self.fan_type,
            list_index: 0,
            missed_polls: 0,
        }
    }

    fn apply(&self, controller: &WirelessController) {
        *controller.master_mac.lock() = self.master_mac;
        *controller.master_channel.lock() = self.channel;
        *controller.discovery_query_channel.lock() = self.channel;
        *controller.discovered_devices.lock() = vec![self.build_device()];
        controller
            .discovery_failures
            .store(0, Ordering::Relaxed);
        match self.mobo_pwm {
            Some(v) => controller.mobo_pwm.store(v as u16, Ordering::Relaxed),
            None => controller.mobo_pwm.store(0xFFFF, Ordering::Relaxed),
        }
    }
}

/// Parse a 42-byte device record from GetDev response.
///
/// Record layout:
/// ```text
/// [0-5]   Device MAC (6 bytes)
/// [6-11]  Master MAC (6 bytes)
/// [12]    RF Channel
/// [13]    RX Type (radio endpoint)
/// [14-17] System time (ms * 0.625)
/// [18]    Device type (0=fan, 65=LC217, 255=master)
/// [19]    Fan count
/// [20-23] Effect index (4 bytes)
/// [24-27] Fan type bytes (4 bytes, per-slot)
/// [28-35] Fan speeds (4x u16 big-endian RPM)
/// [36-39] Current PWM (4 bytes)
/// [40]    Command sequence number
/// [41]    Validation marker (must be 0x1C = 28)
/// ```
fn parse_device_record(data: &[u8], list_index: u8) -> Option<DiscoveredDevice> {
    if data.len() < 42 {
        return None;
    }

    // Validate marker
    if data[41] != 0x1C {
        debug!(
            "  Device record {list_index}: invalid marker 0x{:02x} (expected 0x1C)",
            data[41]
        );
        return None;
    }

    let device_type = data[18];

    // Skip master device (type 0xFF)
    if device_type == 0xFF {
        debug!("  Device record {list_index}: skipping master device");
        return None;
    }

    let mut mac = [0u8; 6];
    mac.copy_from_slice(&data[0..6]);

    let mut master_mac = [0u8; 6];
    master_mac.copy_from_slice(&data[6..12]);

    let channel = data[12];
    let rx_type = data[13];

    let mut fan_types = [0u8; 4];
    fan_types.copy_from_slice(&data[24..28]);

    // Fan RPMs: 4x big-endian u16 at offset 28-35
    let fan_rpms = [
        u16::from_be_bytes([data[28], data[29]]),
        u16::from_be_bytes([data[30], data[31]]),
        u16::from_be_bytes([data[32], data[33]]),
        u16::from_be_bytes([data[34], data[35]]),
    ];

    let mut current_pwm = [0u8; 4];
    current_pwm.copy_from_slice(&data[36..40]);

    let cmd_seq = data[40];
    let reported_fan_count = data[19];
    let inferred_fan_count = fan_types
        .iter()
        .filter(|&&fan_type| fan_type != 0)
        .count()
        .max(fan_rpms.iter().filter(|&&rpm| rpm > 0).count()) as u8;
    let fan_count = match reported_fan_count {
        1..=4 => reported_fan_count,
        _ => inferred_fan_count.min(4),
    };

    if reported_fan_count != fan_count {
        debug!(
            "  Device record {list_index}: adjusted invalid fan count 0x{reported_fan_count:02x} -> {} using slot telemetry",
            fan_count
        );
    }

    // Classify fan type from the first non-zero fan type byte
    let fan_type = fan_types
        .iter()
        .find(|&&b| b != 0)
        .map(|&b| WirelessFanType::from_fan_type_byte(b))
        .unwrap_or(WirelessFanType::Unknown);

    Some(DiscoveredDevice {
        mac,
        master_mac,
        channel,
        rx_type,
        device_type,
        fan_count,
        fan_types,
        fan_rpms,
        current_pwm,
        cmd_seq,
        fan_type,
        list_index,
        missed_polls: 0,
    })
}

fn stabilize_discovery_snapshot(previous: &[DiscoveredDevice], found: &mut [DiscoveredDevice]) {
    for device in found.iter_mut() {
        let Some(previous_device) = previous.iter().find(|candidate| {
            candidate.rx_type == device.rx_type
                && candidate.fan_count == device.fan_count
                && candidate.fan_type == device.fan_type
                && candidate.channel == device.channel
        }) else {
            continue;
        };

        let previous_zero_bytes = previous_device.mac.iter().filter(|&&byte| byte == 0).count();
        let new_zero_bytes = device.mac.iter().filter(|&&byte| byte == 0).count();

        if new_zero_bytes >= previous_zero_bytes + 2 {
            debug!(
                "GetDev: preserving stable MAC {} over suspicious update {}",
                previous_device.mac_str(),
                device.mac_str()
            );
            device.mac = previous_device.mac;
        }
    }
}

fn merge_missing_discovery_devices(
    previous: &[DiscoveredDevice],
    found: &mut Vec<DiscoveredDevice>,
    _master_mac: &[u8; 6],
) {
    if previous.is_empty() || found.is_empty() {
        return;
    }

    for previous_device in previous {
        if is_empty_mac(&previous_device.mac) {
            continue;
        }

        if found.iter().any(|device| device.mac == previous_device.mac) {
            continue;
        }

        let next_miss_count = previous_device.missed_polls.saturating_add(1);
        if next_miss_count > DISCOVERY_DEVICE_GRACE_POLLS {
            debug!(
                "GetDev: dropping stale wireless device {} after {} missed poll(s)",
                previous_device.mac_str(),
                previous_device.missed_polls,
            );
            continue;
        }

        let mut preserved = previous_device.clone();
        preserved.missed_polls = next_miss_count;
        debug!(
            "GetDev: preserving missing wireless device {} for grace poll {}/{}",
            preserved.mac_str(),
            preserved.missed_polls,
            DISCOVERY_DEVICE_GRACE_POLLS,
        );
        found.push(preserved);
    }
}

pub struct WirelessController {
    tx: Option<Arc<Mutex<UsbTransport>>>,
    rx: Option<Arc<Mutex<UsbTransport>>>,
    simulated: Option<SimulatedWireless>,
    poll_stop: Arc<AtomicBool>,
    poll_thread: Option<JoinHandle<()>>,
    video_mode_active: Arc<AtomicBool>,
    master_mac: Arc<Mutex<[u8; 6]>>,
    master_channel: Arc<Mutex<u8>>,
    discovery_query_channel: Arc<Mutex<u8>>,
    discovered_devices: Arc<Mutex<Vec<DiscoveredDevice>>>,
    locally_detached: Arc<Mutex<HashSet<[u8; 6]>>>,
    discovery_failures: Arc<AtomicU16>,
    /// Motherboard PWM duty cycle (0-255) extracted from RX GetDev response bytes [2:3].
    /// 0xFFFF means unavailable/not yet read.
    mobo_pwm: Arc<AtomicU16>,
}

impl Clone for WirelessController {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
            simulated: self.simulated.clone(),
            poll_stop: Arc::clone(&self.poll_stop),
            poll_thread: None,
            video_mode_active: Arc::clone(&self.video_mode_active),
            master_mac: Arc::clone(&self.master_mac),
            master_channel: Arc::clone(&self.master_channel),
            discovery_query_channel: Arc::clone(&self.discovery_query_channel),
            discovered_devices: Arc::clone(&self.discovered_devices),
            locally_detached: Arc::clone(&self.locally_detached),
            discovery_failures: Arc::clone(&self.discovery_failures),
            mobo_pwm: Arc::clone(&self.mobo_pwm),
        }
    }
}

impl WirelessController {
    pub fn new() -> Self {
        let simulated = SimulatedWireless::from_env();
        let controller = Self {
            tx: None,
            rx: None,
            simulated,
            poll_stop: Arc::new(AtomicBool::new(false)),
            poll_thread: None,
            video_mode_active: Arc::new(AtomicBool::new(false)),
            master_mac: Arc::new(Mutex::new([0u8; 6])),
            master_channel: Arc::new(Mutex::new(8)),
            discovery_query_channel: Arc::new(Mutex::new(8)),
            discovered_devices: Arc::new(Mutex::new(Vec::new())),
            locally_detached: Arc::new(Mutex::new(HashSet::new())),
            discovery_failures: Arc::new(AtomicU16::new(0)),
            mobo_pwm: Arc::new(AtomicU16::new(0xFFFF)),
        };

        if let Some(sim) = controller.simulated.as_ref() {
            sim.apply(&controller);
            info!(
                "Wireless simulation enabled: {} ({} fan(s))",
                sim.fan_type.display_name(),
                sim.fan_count
            );
        }

        controller
    }

    pub fn connect(&mut self) -> Result<()> {
        if let Some(sim) = &self.simulated {
            sim.apply(self);
            info!(
                "Wireless simulation active: {} ({} fan(s))",
                sim.fan_type.display_name(),
                sim.fan_count
            );
            return Ok(());
        }

        let mut tx = None;
        let max_retries = 3;

        for attempt in 1..=max_retries {
            match UsbTransport::open(TX_VENDOR, TX_PRODUCT) {
                Ok(device) => {
                    tx = Some(device);
                    break;
                }
                Err(e) if attempt < max_retries => {
                    debug!("TX device not found (attempt {attempt}/{max_retries}): {e}");
                    thread::sleep(Duration::from_millis(1000 * attempt as u64));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(e))
                        .context("opening wireless TX (0416:8040)");
                }
            }
        }

        let mut tx = tx.context("TX device failed to open after retries")?;
        tx.detach_and_configure("TX")?;
        let tx_arc = Arc::new(Mutex::new(tx));

        let rx_arc = match UsbTransport::open(RX_VENDOR, RX_PRODUCT) {
            Ok(mut rx) => {
                rx.detach_and_configure("RX")?;
                Some(Arc::new(Mutex::new(rx)))
            }
            Err(_) => {
                warn!("RX device (0416:8041) not found – telemetry disabled");
                None
            }
        };

        self.tx = Some(tx_arc);
        self.rx = rx_arc;

        self.discover_master_mac()?;
        Ok(())
    }

    /// Discovers master MAC address and channel by querying TX with USB_GetMac.
    ///
    /// Tries the default channel first, then scans.
    /// Channels should be even numbers.
    fn discover_master_mac(&self) -> Result<()> {
        let tx = self.tx.as_ref().context("TX device not available")?;
        info!("Discovering master MAC address and wireless channel...");

        // Try default (8) first, then even channels 2-38, then odd as fallback
        let channels_to_try: Vec<u8> = std::iter::once(8u8)
            .chain((2..=38).filter(|&ch| ch != 8 && ch % 2 == 0))
            .chain((1..=39).filter(|&ch| ch % 2 == 1))
            .collect();

        for channel in channels_to_try {
            let mut cmd = vec![0u8; 64];
            cmd[0] = USB_CMD_GET_MAC;
            cmd[1] = channel;

            let handle = tx.lock();
            if handle.write_bulk(&cmd, USB_TIMEOUT).is_err() {
                drop(handle);
                continue;
            }

            let mut response = [0u8; 64];
            let len = match handle.read_bulk(&mut response, Duration::from_millis(500)) {
                Ok(len) => len,
                Err(_) => {
                    drop(handle);
                    continue;
                }
            };
            drop(handle);

            // Response: [0]=0x11, [1-6]=master MAC, [7-10]=sysTime, [11-12]=fwVer
            if len >= 7 && response[0] == USB_CMD_GET_MAC {
                let mut mac = self.master_mac.lock();
                mac.copy_from_slice(&response[1..7]);
                if mac.iter().any(|&b| b != 0) {
                    *self.discovery_query_channel.lock() = channel;
                    info!(
                        "Master MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} query_channel={}",
                        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], channel
                    );
                    if len >= 13 {
                        let fw_ver = u16::from_be_bytes([response[11], response[12]]);
                        debug!("Master firmware version: {fw_ver}");
                    }
                    return Ok(());
                }
            }
        }

        bail!("Failed to discover master MAC on any channel (tried 1-39)");
    }

    pub fn start_polling(&mut self) -> Result<()> {
        if let Some(sim) = &self.simulated {
            sim.apply(self);
            return Ok(());
        }

        let tx = self
            .tx
            .as_ref()
            .cloned()
            .context("TX device must be connected before polling")?;
        let rx = self
            .rx
            .as_ref()
            .cloned()
            .context("RX device must be connected for device discovery")?;

        {
            let handle = tx.lock();
            handle
                .write_bulk(&CMD_RESET, USB_TIMEOUT)
                .context("sending TX reset")?;
        }

        self.prime_rx_for_discovery()?;

        self.video_mode_active.store(false, Ordering::Release);
        self.poll_stop.store(false, Ordering::SeqCst);

        let stop_flag = self.poll_stop.clone();
        let discovered_devices = Arc::clone(&self.discovered_devices);
        let mobo_pwm = Arc::clone(&self.mobo_pwm);
        let master_mac = Arc::clone(&self.master_mac);
        let master_channel = Arc::clone(&self.master_channel);
        let discovery_query_channel = Arc::clone(&self.discovery_query_channel);
        let discovery_failures = Arc::clone(&self.discovery_failures);

        self.poll_thread = Some(thread::spawn(move || {
            let mut found_devices = false;
            while !stop_flag.load(Ordering::SeqCst) {
                if let Err(err) = poll_and_discover(
                    &rx,
                    Some(&tx),
                    &discovered_devices,
                    &mobo_pwm,
                    &master_mac,
                    &master_channel,
                    &discovery_query_channel,
                    &discovery_failures,
                    false,
                ) {
                    warn!("RX polling error: {err:?}");
                    break;
                }
                if !found_devices && !discovered_devices.lock().is_empty() {
                    found_devices = true;
                }
                let interval = if found_devices { 5000 } else { 500 };
                thread::sleep(Duration::from_millis(interval));
            }
        }));

        thread::sleep(Duration::from_millis(1500));
        Ok(())
    }

    fn prime_rx_for_discovery(&self) -> Result<()> {
        if self.simulated.is_some() {
            return Ok(());
        }

        let rx = self
            .rx
            .as_ref()
            .context("RX device must be connected for discovery priming")?;

        drain_rx_buffer(rx);
        self.send_rx_sequence()?;
        thread::sleep(RX_PRIME_SETTLE);
        drain_rx_buffer(rx);
        Ok(())
    }

    pub fn ensure_video_mode(&self) -> Result<()> {
        if self.simulated.is_some() {
            return Ok(());
        }

        if self.video_mode_active.load(Ordering::Acquire) {
            return Ok(());
        }

        if let Some(tx) = &self.tx {
            let handle = tx.lock();
            handle
                .write_bulk(&CMD_VIDEO_START, USB_TIMEOUT)
                .context("sending TX video start")?;
            thread::sleep(Duration::from_millis(2));

            let devices = self.discovered_devices.lock();
            let device_count = devices.len().max(1);
            let master_ch = *self.master_channel.lock();

            for device_idx in 0..device_count {
                let mut cmd = vec![0u8; 64];
                cmd[0] = USB_CMD_SEND_RF;
                cmd[1] = device_idx as u8;
                cmd[2] = master_ch;
                cmd[3] = 0xFF; // Prep marker
                handle
                    .write_bulk(&cmd, USB_TIMEOUT)
                    .context("sending TX prep command")?;
                thread::sleep(Duration::from_millis(1));
            }

            drop(handle);
            self.video_mode_active.store(true, Ordering::Release);
            info!("Video mode activated with {device_count} device(s)");
        }
        Ok(())
    }

    pub fn send_rx_sequence(&self) -> Result<()> {
        if self.simulated.is_some() {
            return Ok(());
        }

        if let Some(rx) = &self.rx {
            for (cmd, capture) in [
                (&*CMD_RX_QUERY_34, true),
                (&*CMD_RX_QUERY_37, true),
                (&*CMD_RX_LCD_MODE, false),
            ] {
                {
                    let handle = rx.lock();
                    handle
                        .write_bulk(cmd, USB_TIMEOUT)
                        .context("sending RX command")?;
                }
                thread::sleep(Duration::from_millis(2));
                if capture {
                    let mut buf = [0u8; 64];
                    let handle = rx.lock();
                    if let Ok(len) = handle.read_bulk(&mut buf, USB_TIMEOUT) {
                        debug!("RX resp: {:02x?}", &buf[..len.min(8)]);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn soft_reset(&mut self) -> bool {
        if let Some(sim) = &self.simulated {
            sim.apply(self);
            return true;
        }

        if self.tx.is_none() {
            if let Ok(mut transport) = UsbTransport::open(TX_VENDOR, TX_PRODUCT) {
                if transport.detach_and_configure("TX").is_ok() {
                    self.tx = Some(Arc::new(Mutex::new(transport)));
                }
            }
        }

        if let Some(tx) = &self.tx {
            {
                let handle = tx.lock();
                if handle.write_bulk(&CMD_RESET, USB_TIMEOUT).is_err() {
                    return false;
                }
            }
            self.video_mode_active.store(false, Ordering::Release);
            thread::sleep(Duration::from_millis(50));
            return self.ensure_video_mode().is_ok();
        }

        false
    }

    /// Whether any wireless devices have been discovered.
    pub fn has_discovered_devices(&self) -> bool {
        !self.discovered_devices.lock().is_empty()
    }

    /// Number of discovered wireless devices.
    pub fn discovered_device_count(&self) -> usize {
        self.discovered_devices.lock().len()
    }

    /// Get a snapshot of all discovered devices.
    pub fn devices(&self) -> Vec<DiscoveredDevice> {
        self.discovered_devices.lock().clone()
    }

    /// Whether the app has locally detached this device from active control.
    pub fn is_locally_detached(&self, mac: &[u8; 6]) -> bool {
        self.locally_detached.lock().contains(mac)
    }

    /// Get a snapshot of a single device by its MAC address.
    pub fn device_by_mac(&self, mac: &[u8; 6]) -> Option<DiscoveredDevice> {
        self.discovered_devices
            .lock()
            .iter()
            .find(|d| &d.mac == mac)
            .cloned()
    }

    /// Get the current motherboard PWM duty cycle (0-255), or None if unavailable.
    ///
    /// Extracted from the RX GetDev response bytes [2:3] during polling.
    /// Returns None if the high bit of byte[2] is set (mobo PWM not available)
    /// or if no polling data has been received yet.
    pub fn motherboard_pwm(&self) -> Option<u8> {
        match self.mobo_pwm.load(Ordering::Relaxed) {
            0xFFFF => None,
            v => Some(v as u8),
        }
    }

    pub fn master_mac(&self) -> [u8; 6] {
        *self.master_mac.lock()
    }

    pub fn refresh_discovery(&self) -> Result<usize> {
        if let Some(sim) = &self.simulated {
            sim.apply(self);
            return Ok(self.discovered_device_count());
        }

        let rx = self
            .rx
            .as_ref()
            .context("RX device must be connected for discovery refresh")?;
        let tx = self.tx.as_ref();

        self.prime_rx_for_discovery()?;
        poll_and_discover(
            rx,
            tx,
            &self.discovered_devices,
            &self.mobo_pwm,
            &self.master_mac,
            &self.master_channel,
            &self.discovery_query_channel,
            &self.discovery_failures,
            true,
        )?;

        Ok(self.discovered_device_count())
    }

    pub fn bind_device(&self, device_id: &str) -> Result<()> {
        let device = self
            .discovered_devices
            .lock()
            .iter()
            .find(|candidate| format!("wireless:{}", candidate.mac_str()) == device_id)
            .cloned()
            .context(format!("Wireless device not found in discovery: {device_id}"))?;

        let master_mac = *self.master_mac.lock();
        if is_empty_mac(&master_mac) {
            bail!("wireless master MAC not initialized");
        }

        if device.master_mac == master_mac {
            let was_locally_detached = self.locally_detached.lock().remove(&device.mac);
            if was_locally_detached {
                info!(
                    "Reconnected locally detached wireless device {} to the current dongle",
                    device.mac_str()
                );
            } else {
                debug!(
                    "Wireless device {} is already connected to the current dongle",
                    device.mac_str()
                );
            }
            return Ok(());
        }

        if !is_empty_mac(&device.master_mac) && device.master_mac != master_mac {
            bail!(
                "wireless device {} is already paired to another controller",
                device.mac_str()
            );
        }

        let master_channel = *self.master_channel.lock();
        let target_rx_type = self
            .next_available_rx_type(Some(&device.mac))
            .context("no free wireless RX slot available for bind")?;
        let slave_index = self.next_bind_index(Some(&device.mac));

        if self.simulated.is_some() {
            let mut devices = self.discovered_devices.lock();
            if let Some(candidate) = devices.iter_mut().find(|candidate| candidate.mac == device.mac)
            {
                candidate.master_mac = master_mac;
                candidate.channel = master_channel;
                candidate.rx_type = target_rx_type;
            }
            self.locally_detached.lock().remove(&device.mac);
            info!(
                "[sim] Bound wireless device {} to current master (rx={}, slot={})",
                device.mac_str(),
                target_rx_type,
                slave_index
            );
            return Ok(());
        }

        let tx = self.tx.as_ref().context("TX device not connected")?;

        let mut rf_data = vec![0u8; RF_DATA_SIZE];
        rf_data[0] = RF_SELECT;
        rf_data[1] = RF_PWM_CMD;
        rf_data[2..8].copy_from_slice(&device.mac);
        rf_data[8..14].copy_from_slice(&master_mac);
        rf_data[14] = target_rx_type;
        rf_data[15] = master_channel;
        rf_data[16] = slave_index;
        rf_data[17..21].copy_from_slice(&device.current_pwm);

        let handle = tx.lock();
        send_control_rf_payload(
            &handle,
            device.channel,
            device.rx_type,
            &rf_data,
            FAN_PWM_SEND_REPEATS,
            FAN_PWM_REPEAT_GAP,
            "wireless bind",
        )?;
        drop(handle);

        {
            let mut devices = self.discovered_devices.lock();
            if let Some(candidate) = devices.iter_mut().find(|candidate| candidate.mac == device.mac)
            {
                candidate.master_mac = master_mac;
                candidate.channel = master_channel;
                candidate.rx_type = target_rx_type;
            }
        }
        self.locally_detached.lock().remove(&device.mac);

        info!(
            "Bound wireless device {} to current master (rx={}, slot={})",
            device.mac_str(),
            target_rx_type,
            slave_index
        );
        Ok(())
    }

    /// Disconnect a discovered wireless device from the active dongle/controller.
    ///
    /// This mirrors L-Connect's RF unbind flow by sending a control frame with:
    /// - empty target master MAC
    /// - target RX type `0`
    /// - target channel = current master channel
    /// - slave index `0`
    /// - current PWM bytes kept intact
    pub fn unbind_device(&self, device_id: &str) -> Result<()> {
        let device = self
            .discovered_devices
            .lock()
            .iter()
            .find(|candidate| format!("wireless:{}", candidate.mac_str()) == device_id)
            .cloned()
            .context(format!("Wireless device not found in discovery: {device_id}"))?;

        if self.simulated.is_some() {
            let mut devices = self.discovered_devices.lock();
            if let Some(candidate) = devices.iter_mut().find(|candidate| candidate.mac == device.mac)
            {
                candidate.master_mac = [0; 6];
                candidate.rx_type = 0;
                candidate.missed_polls = 0;
            }
            self.locally_detached.lock().insert(device.mac);
            info!("[sim] Unbound wireless device {}", device.mac_str());
            return Ok(());
        }

        let tx = self.tx.as_ref().context("TX device not connected")?;
        let master_channel = *self.master_channel.lock();

        let mut rf_data = vec![0u8; RF_DATA_SIZE];
        rf_data[0] = RF_SELECT;
        rf_data[1] = RF_PWM_CMD;
        rf_data[2..8].copy_from_slice(&device.mac);
        rf_data[14] = 0;
        rf_data[15] = master_channel;
        rf_data[16] = 0;
        rf_data[17..21].copy_from_slice(&device.current_pwm);

        let handle = tx.lock();
        send_control_rf_payload(
            &handle,
            device.channel,
            device.rx_type,
            &rf_data,
            FAN_PWM_SEND_REPEATS,
            FAN_PWM_REPEAT_GAP,
            "wireless unbind",
        )?;
        drop(handle);
        self.locally_detached.lock().insert(device.mac);

        info!("Sent wireless unbind request for {}", device.mac_str());

        Ok(())
    }

    /// Set fan PWM values for a specific device identified by MAC address.
    ///
    /// Uses the device's own rx_type and channel from discovery, not a global
    /// value.
    ///
    /// ## RF PWM packet layout (240 bytes):
    /// ```text
    /// [0]     = 0x12 (RF_Select — envelope command)
    /// [1]     = 0x10 (RF_Bind — PWM sub-command)
    /// [2-7]   = Device (slave) MAC address
    /// [8-13]  = Master MAC address
    /// [14]    = Target RX type (from device discovery)
    /// [15]    = Target channel (master channel)
    /// [16]    = Sequence index (1 for one-shot commands)
    /// [17-20] = Fan PWM values (4 bytes, one per fan slot)
    /// [21-239]= Reserved
    /// ```
    pub fn set_fan_speeds_by_mac(&self, mac: &[u8; 6], fan_pwm: &[u8; 4]) -> Result<()> {
        if self.simulated.is_some() {
            let mut devices = self.discovered_devices.lock();
            let device = devices
                .iter_mut()
                .find(|d| &d.mac == mac)
                .context(format!(
                    "Simulated device MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} not found",
                    mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
                ))?;

            let mut pwm = *fan_pwm;
            apply_pwm_constraints(&mut pwm, device);
            device.current_pwm = pwm;
            device.cmd_seq = device.cmd_seq.wrapping_add(1);

            let max_rpm: u16 = 2000;
            for idx in 0..4 {
                if idx as u8 >= device.fan_count {
                    device.fan_rpms[idx] = 0;
                    continue;
                }
                let rpm = (pwm[idx] as u32 * max_rpm as u32 / 255) as u16;
                device.fan_rpms[idx] = rpm;
            }

            debug!(
                "[sim] Set fan PWM for {}: {:?}",
                device.mac_str(),
                pwm
            );
            return Ok(());
        }

        let tx = self.tx.as_ref().context("TX device not connected")?;

        let device = self.discovered_devices
            .lock()
            .iter()
            .find(|d| &d.mac == mac)
            .cloned()
            .context(format!(
                "Device MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} not found in discovery",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
            ))?;

        let master_mac = *self.master_mac.lock();
        let master_ch = *self.master_channel.lock();

        // Apply minimum duty enforcement and CLV1 PWM filter
        let mut pwm = *fan_pwm;
        apply_pwm_constraints(&mut pwm, &device);

        // Build RF PWM packet (240 bytes)
        let mut rf_data = vec![0u8; RF_DATA_SIZE];
        rf_data[0] = RF_SELECT;            // RF_Select envelope command
        rf_data[1] = RF_PWM_CMD;           // PWM sub-command (0x10)
        rf_data[2..8].copy_from_slice(&device.mac);
        rf_data[8..14].copy_from_slice(&master_mac);
        rf_data[14] = device.rx_type;      // Per-device RX type from discovery
        rf_data[15] = master_ch;           // Target channel = master channel
        rf_data[16] = 1;                   // Sequence index (1 for one-shot)
        rf_data[17..21].copy_from_slice(&pwm);

        let handle = tx.lock();
        send_control_rf_payload(
            &handle,
            device.channel,
            device.rx_type,
            &rf_data,
            FAN_PWM_SEND_REPEATS,
            FAN_PWM_REPEAT_GAP,
            "fan speed",
        )?;

        debug!(
            "Set fan PWM for {} (rx={}, ch={}, repeats={}): {:?}",
            device.mac_str(),
            device.rx_type,
            device.channel,
            FAN_PWM_SEND_REPEATS,
            pwm
        );

        // Keep our local discovery snapshot aligned with the last successful
        // write so transient RX misses do not trigger unnecessary re-sends.
        let mut devices = self.discovered_devices.lock();
        if let Some(device_state) = devices.iter_mut().find(|d| d.mac == device.mac) {
            device_state.current_pwm = pwm;
            device_state.cmd_seq = device_state.cmd_seq.wrapping_add(1);
        }
        Ok(())
    }

    /// Set fan PWM values by device list index (backward compat with old API).
    ///
    /// Index corresponds to the position in the discovery list (0-based).
    pub fn set_fan_speeds(&self, device_index: u8, fan_pwm: &[u8; 4]) -> Result<()> {
        let mac = {
            let devices = self.discovered_devices.lock();
            devices
                .iter()
                .find(|d| d.list_index == device_index)
                .map(|d| d.mac)
                .context(format!(
                    "No device at index {device_index} (discovered {} device(s))",
                    devices.len()
                ))?
        };

        self.set_fan_speeds_by_mac(&mac, fan_pwm)
    }

    /// Send a single frame of per-LED RGB colors to a wireless device.
    ///
    /// Wrapper around `send_rgb_frames` for single-frame (static/direct) use.
    pub fn send_rgb_direct(
        &self,
        mac: &[u8; 6],
        colors: &[[u8; 3]],
        effect_index: &[u8; 4],
        header_repeats: u8,
    ) -> Result<()> {
        if self.simulated.is_some() {
            debug!(
                "[sim] RGB direct for {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} ({} colors, effect={:02x?})",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
                colors.len(),
                effect_index
            );
            return Ok(());
        }

        let led_num = colors.len() as u8;
        let mut raw_rgb = Vec::with_capacity(colors.len() * 3);
        for color in colors {
            raw_rgb.extend_from_slice(color);
        }
        self.send_rgb_payload(mac, &raw_rgb, led_num, 1, 5000, effect_index, header_repeats)
    }

    /// Send a multi-frame animation to a wireless device.
    ///
    /// Firmware stores the compressed blob and loops all frames at `interval_ms`.
    /// Used for batched OpenRGB streaming — collect N frames, send once, let
    /// firmware play them back smoothly with zero host involvement.
    pub fn send_rgb_frames(
        &self,
        mac: &[u8; 6],
        frames: &[Vec<[u8; 3]>],
        interval_ms: u16,
        effect_index: &[u8; 4],
        header_repeats: u8,
    ) -> Result<()> {
        if self.simulated.is_some() {
            debug!(
                "[sim] RGB frames for {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} ({} frame(s), interval={}ms, effect={:02x?})",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
                frames.len(),
                interval_ms,
                effect_index
            );
            return Ok(());
        }

        if frames.is_empty() {
            return Ok(());
        }
        let led_num = frames[0].len() as u8;
        let total_frames = frames.len() as u16;

        let mut raw_rgb = Vec::with_capacity(frames.len() * led_num as usize * 3);
        for frame in frames {
            for color in frame {
                raw_rgb.extend_from_slice(color);
            }
        }

        self.send_rgb_payload(mac, &raw_rgb, led_num, total_frames, interval_ms, effect_index, header_repeats)
    }

    /// Core RF RGB payload sender.
    ///
    /// Compresses raw RGB data, splits into 220-byte chunks, and sends via RF.
    /// Header packet (index=0) carries metadata and is repeated for reliability.
    /// Data packets (index=1..N) carry compressed data chunks.
    fn send_rgb_payload(
        &self,
        mac: &[u8; 6],
        raw_rgb: &[u8],
        led_num: u8,
        total_frames: u16,
        interval_ms: u16,
        effect_index: &[u8; 4],
        header_repeats: u8,
    ) -> Result<()> {
        if self.simulated.is_some() {
            debug!(
                "[sim] RGB payload for {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} ({} frames, {} LEDs, interval={}ms, effect={:02x?})",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
                total_frames,
                led_num,
                interval_ms,
                effect_index
            );
            return Ok(());
        }

        let tx = self.tx.as_ref().context("TX device not connected")?;

        let device = self
            .discovered_devices
            .lock()
            .iter()
            .find(|d| &d.mac == mac)
            .cloned()
            .context("device not found for RGB send")?;

        let master_mac = *self.master_mac.lock();

        let compressed = crate::tinyuz::compress(raw_rgb)
            .context("failed to compress RGB data")?;

        const LZO_RF_VALID_LEN: usize = 220;
        let total_pk_num =
            (compressed.len() as f64 / LZO_RF_VALID_LEN as f64).ceil() as u8;

        let mut offset: usize = 0;
        let mut index: u8 = 0;

        // Hold TX lock for the entire transfer to prevent interleaving
        // with PWM or other TX operations.
        let handle = tx.lock();

        while offset < compressed.len() || index == 0 {
            let mut rf_data = vec![0u8; RF_DATA_SIZE];

            rf_data[0] = RF_SELECT;
            rf_data[1] = RF_SET_RGB;
            rf_data[2..8].copy_from_slice(&device.mac);
            rf_data[8..14].copy_from_slice(&master_mac);
            rf_data[14..18].copy_from_slice(effect_index);
            rf_data[18] = index;
            rf_data[19] = total_pk_num + 1;

            if index == 0 {
                // Header packet: metadata
                let data_len = compressed.len() as u32;
                rf_data[20] = (data_len >> 24) as u8;
                rf_data[21] = ((data_len >> 16) & 0xFF) as u8;
                rf_data[22] = ((data_len >> 8) & 0xFF) as u8;
                rf_data[23] = (data_len & 0xFF) as u8;
                rf_data[24] = 0;
                rf_data[25] = (total_frames >> 8) as u8;
                rf_data[26] = (total_frames & 0xFF) as u8;
                rf_data[27] = led_num;
                rf_data[32] = (interval_ms >> 8) as u8;
                rf_data[33] = (interval_ms & 0xFF) as u8;

                let repeats = header_repeats.max(1);
                let gap_ms = if repeats <= 2 { 2 } else { 20 };
                for repeat in 0..repeats {
                    self.send_rf_packet(&handle, &device, &rf_data)?;
                    if repeat < repeats - 1 {
                        thread::sleep(Duration::from_millis(gap_ms));
                    }
                }
            } else {
                // Data packet: 220 bytes of compressed data
                let remaining = compressed.len() - offset;
                let chunk_len = remaining.min(LZO_RF_VALID_LEN);
                rf_data[20..20 + chunk_len]
                    .copy_from_slice(&compressed[offset..offset + chunk_len]);
                offset += LZO_RF_VALID_LEN;

                self.send_rf_packet(&handle, &device, &rf_data)?;
            }

            index += 1;
        }

        drop(handle);

        debug!(
            "Sent RGB to {} ({} frame(s), {} LEDs, {} compressed, {} packets, {}ms interval)",
            device.mac_str(), total_frames, led_num, compressed.len(), index, interval_ms
        );
        Ok(())
    }

    /// Send a 240-byte RF packet as 4× 64-byte USB chunks.
    fn send_rf_packet(
        &self,
        handle: &UsbTransport,
        device: &DiscoveredDevice,
        rf_data: &[u8],
    ) -> Result<()> {
        for chunk_idx in 0..RF_CHUNKS as u8 {
            let mut packet = vec![0u8; 64];
            packet[0] = USB_CMD_SEND_RF;
            packet[1] = chunk_idx;
            packet[2] = device.channel;
            packet[3] = device.rx_type;

            let start = chunk_idx as usize * RF_CHUNK_SIZE;
            let end = start + RF_CHUNK_SIZE;
            packet[4..64].copy_from_slice(&rf_data[start..end]);

            handle
                .write_bulk(&packet, USB_TIMEOUT)
                .context("sending RGB RF packet")?;
            thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        self.poll_stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.poll_thread.take() {
            let _ = handle.join();
        }
        self.tx.take();
        self.rx.take();
    }

    fn next_available_rx_type(&self, exclude_mac: Option<&[u8; 6]>) -> Option<u8> {
        let master_mac = *self.master_mac.lock();
        let devices = self.discovered_devices.lock();

        let mut max_used = 1u8;
        for device in devices.iter() {
            if !is_bound_to_master(device, &master_mac, exclude_mac) {
                continue;
            }
            max_used = max_used.max(device.rx_type);
        }

        for candidate in max_used..=14 {
            if rx_type_is_free(&devices, &master_mac, exclude_mac, candidate) {
                return Some(candidate);
            }
        }

        for candidate in 1..=14 {
            if rx_type_is_free(&devices, &master_mac, exclude_mac, candidate) {
                return Some(candidate);
            }
        }

        None
    }

    fn next_bind_index(&self, exclude_mac: Option<&[u8; 6]>) -> u8 {
        let master_mac = *self.master_mac.lock();
        let devices = self.discovered_devices.lock();
        let bound_count = devices
            .iter()
            .filter(|device| is_bound_to_master(device, &master_mac, exclude_mac))
            .count() as u8;
        bound_count.saturating_add(1).max(1)
    }
}

impl Default for WirelessController {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WirelessController {
    fn drop(&mut self) {
        self.stop();
    }
}

fn send_control_rf_payload(
    handle: &UsbTransport,
    channel: u8,
    rx_type: u8,
    rf_data: &[u8],
    repeats: u8,
    repeat_gap: Duration,
    context: &str,
) -> Result<()> {
    for repeat in 0..repeats {
        for chunk_idx in 0..RF_CHUNKS as u8 {
            let mut packet = vec![0u8; 64];
            packet[0] = USB_CMD_SEND_RF;
            packet[1] = chunk_idx;
            packet[2] = channel;
            packet[3] = rx_type;

            let start = chunk_idx as usize * RF_CHUNK_SIZE;
            let end = start + RF_CHUNK_SIZE;
            packet[4..64].copy_from_slice(&rf_data[start..end]);

            handle
                .write_bulk(&packet, USB_TIMEOUT)
                .with_context(|| format!("sending {context} RF packet"))?;
            thread::sleep(Duration::from_millis(1));
        }

        if repeat + 1 < repeats {
            thread::sleep(repeat_gap);
        }
    }

    Ok(())
}

/// Apply minimum duty enforcement and CLV1 PWM filter.
///
/// Enforces per-fan-type minimums and special PWM remapping
/// for CLV1 devices (values 153-155 to 152/156).
fn apply_pwm_constraints(pwm: &mut [u8; 4], device: &DiscoveredDevice) {
    let min_pwm = ((device.fan_type.min_duty_percent() as f32 / 100.0) * 255.0) as u8;

    for (i, val) in pwm.iter_mut().enumerate() {
        // Only apply to slots that have fans (based on fan_count)
        if i as u8 >= device.fan_count {
            *val = 0; // Unused slots must be 0
            continue;
        }

        // Enforce minimum PWM
        if *val > 0 && *val < min_pwm {
            *val = min_pwm;
        }

        // CLV1 special PWM filter
        if device.fan_type == WirelessFanType::Clv1 {
            match *val {
                153 | 154 => *val = 152,
                155 => *val = 156,
                _ => {}
            }
        }
    }
}

/// Polls the RX device for the current device list.
///
/// Current firmware expects GetDev in the form `[0x10, channel, ...]`
/// rather than the older `[0x10, page=1, ...]` shape.
fn poll_and_discover(
    rx: &Arc<Mutex<UsbTransport>>,
    tx: Option<&Arc<Mutex<UsbTransport>>>,
    discovered_devices: &Arc<Mutex<Vec<DiscoveredDevice>>>,
    mobo_pwm: &Arc<AtomicU16>,
    master_mac: &Arc<Mutex<[u8; 6]>>,
    master_channel: &Arc<Mutex<u8>>,
    discovery_query_channel: &Arc<Mutex<u8>>,
    discovery_failures: &Arc<AtomicU16>,
    force_full_scan: bool,
) -> Result<()> {
    let seeded_channel = *discovery_query_channel.lock();
    let had_devices = !discovered_devices.lock().is_empty();
    let fallback_allowed = force_full_scan
        || !had_devices
        || discovery_failures.load(Ordering::Relaxed) >= DISCOVERY_RESCAN_THRESHOLD;
    let channels = if fallback_allowed {
        preferred_scan_channels(seeded_channel)
    } else {
        vec![seeded_channel]
    };

    for channel in channels {
        let mut cmd = vec![0u8; 64];
        cmd[0] = USB_CMD_SEND_RF;
        cmd[1] = channel;

        let handle = rx.lock();
        handle
            .write_bulk(&cmd, USB_TIMEOUT)
            .with_context(|| format!("sending GetDev command on channel {channel}"))?;

        let mut response = [0u8; 512];
        match handle.read_bulk(&mut response, GET_DEV_TIMEOUT) {
            Ok(len) if len >= 4 => {
                if response[0] != USB_CMD_SEND_RF {
                    debug!(
                        "GetDev[ch={channel}]: unexpected response 0x{:02x}",
                        response[0]
                    );
                    continue;
                }

                let device_count = response[1] as usize;
                debug!("GetDev[ch={channel}]: {device_count} device slot(s) reported");

                if device_count == 0 || device_count > 12 {
                    continue;
                }

                let indicator = response[2];
                if indicator >> 7 == 1 {
                    mobo_pwm.store(0xFFFF, Ordering::Relaxed);
                } else {
                    let off_time = (indicator & 0x7F) as u16;
                    let on_time = response[3] as u16;
                    let denominator = off_time + on_time;
                    if denominator > 0 {
                        let pwm = (255u16 * on_time / denominator).min(255);
                        mobo_pwm.store(pwm, Ordering::Relaxed);
                    } else {
                        mobo_pwm.store(0xFFFF, Ordering::Relaxed);
                    }
                }

                let mut found = Vec::new();
                let mut offset = 4;

                for idx in 0..device_count {
                    if offset + 42 > len {
                        debug!("GetDev[ch={channel}]: response truncated at device {idx}");
                        break;
                    }

                    if let Some(device) =
                        parse_device_record(&response[offset..offset + 42], idx as u8)
                    {
                        debug!(
                            "  [ch={channel} idx={idx}] {} type=0x{:02x} fans={} RPM=[{},{},{},{}] PWM=[{},{},{},{}]",
                            device,
                            device.device_type,
                            device.fan_count,
                            device.fan_rpms[0],
                            device.fan_rpms[1],
                            device.fan_rpms[2],
                            device.fan_rpms[3],
                            device.current_pwm[0],
                            device.current_pwm[1],
                            device.current_pwm[2],
                            device.current_pwm[3],
                        );
                        found.push(device);
                    }

                    offset += 42;
                }

                if found.is_empty() {
                    debug!("GetDev[ch={channel}]: no parseable wireless devices");
                    continue;
                }

                let previous_devices = discovered_devices.lock().clone();
                stabilize_discovery_snapshot(&previous_devices, &mut found);
                let current_master_mac = *master_mac.lock();
                merge_missing_discovery_devices(&previous_devices, &mut found, &current_master_mac);

                let reported_rf_channel = found.first().map(|device| device.channel).filter(|&candidate| {
                    candidate > 0 && found.iter().all(|device| device.channel == candidate)
                });

                {
                    let mut devices = discovered_devices.lock();
                    let old_count = devices.len();
                    *devices = found;
                    if old_count != devices.len() {
                        info!("Discovered {} wireless device(s)", devices.len());
                    }
                }

                discovery_failures.store(0, Ordering::Relaxed);

                if let Some(rf_channel) = reported_rf_channel {
                    let mut current_rf_channel = master_channel.lock();
                    if *current_rf_channel != rf_channel {
                        *current_rf_channel = rf_channel;
                        info!("Wireless RF channel confirmed as {rf_channel}");
                    }
                }

                if channel != seeded_channel {
                    *discovery_query_channel.lock() = channel;
                    debug!("Wireless discovery query locked onto channel {channel}");
                }
                return Ok(());
            }
            Ok(len) => {
                debug!("GetDev[ch={channel}]: short response len={len} bytes");
            }
            Err(lianli_transport::TransportError::Usb(rusb::Error::Timeout)) => {
                debug!("GetDev[ch={channel}]: read timeout waiting for response");
            }
            Err(err) => {
                debug!("GetDev[ch={channel}] error: {err}");
            }
        }
    }

    let failure_count = discovery_failures
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);

    if had_devices && !fallback_allowed {
        debug!(
            "GetDev[ch={seeded_channel}]: keeping previous discovery after transient miss ({failure_count}/{DISCOVERY_RESCAN_THRESHOLD})"
        );
        return Ok(());
    }

    if let Some(tx) = tx {
        let mut cmd = vec![0u8; 64];
        cmd[0] = USB_CMD_SEND_RF;
        cmd[1] = seeded_channel;

        let handle = tx.lock();
        if let Err(err) = handle.write_bulk(&cmd, USB_TIMEOUT) {
            debug!("GetDev[TX fallback]: write error: {err}");
        } else {
            let mut tx_response = [0u8; 512];
            match handle.read_bulk(&mut tx_response, GET_DEV_TIMEOUT) {
                Ok(len) => {
                    debug!(
                        "GetDev[TX fallback]: response len={len} header={:02x?}",
                        &tx_response[..len.min(16)]
                    );
                }
                Err(lianli_transport::TransportError::Usb(rusb::Error::Timeout)) => {
                    debug!("GetDev[TX fallback]: read timeout waiting for response");
                }
                Err(err) => {
                    debug!("GetDev[TX fallback] error: {err}");
                }
            }
        }
    }

    if had_devices {
        debug!(
            "GetDev: keeping previous discovery after unsuccessful rescan ({failure_count} consecutive miss(es))"
        );
    }

    Ok(())
}

fn drain_rx_buffer(rx: &Arc<Mutex<UsbTransport>>) {
    let mut discarded = 0usize;
    let mut buf = [0u8; 64];
    let handle = rx.lock();

    loop {
        match handle.read_bulk(&mut buf, RX_DRAIN_TIMEOUT) {
            Ok(len) => {
                discarded += 1;
                debug!("RX drain: discarded stale packet {:02x?}", &buf[..len.min(8)]);
                if discarded >= 8 {
                    break;
                }
            }
            Err(lianli_transport::TransportError::Usb(rusb::Error::Timeout)) => break,
            Err(err) => {
                debug!("RX drain error: {err}");
                break;
            }
        }
    }

    if discarded > 0 {
        debug!("RX drain: discarded {discarded} stale packet(s)");
    }
}

fn is_empty_mac(mac: &[u8; 6]) -> bool {
    mac.iter().all(|&byte| byte == 0)
}

fn is_bound_to_master(
    device: &DiscoveredDevice,
    master_mac: &[u8; 6],
    exclude_mac: Option<&[u8; 6]>,
) -> bool {
    if exclude_mac.is_some_and(|candidate| candidate == &device.mac) {
        return false;
    }

    !is_empty_mac(&device.master_mac) && &device.master_mac == master_mac
}

fn rx_type_is_free(
    devices: &[DiscoveredDevice],
    master_mac: &[u8; 6],
    exclude_mac: Option<&[u8; 6]>,
    candidate: u8,
) -> bool {
    devices.iter().all(|device| {
        !is_bound_to_master(device, master_mac, exclude_mac) || device.rx_type != candidate
    })
}

#[cfg(test)]
mod tests {
    use super::{
        merge_missing_discovery_devices, DiscoveredDevice, WirelessFanType,
        DISCOVERY_DEVICE_GRACE_POLLS,
    };

    fn test_device(mac_tail: u8, master_mac: [u8; 6], missed_polls: u8) -> DiscoveredDevice {
        DiscoveredDevice {
            mac: [0xaa, 0xbb, 0xcc, 0xdd, 0xee, mac_tail],
            master_mac,
            channel: 8,
            rx_type: 1,
            device_type: 0,
            fan_count: 1,
            fan_types: [36, 0, 0, 0],
            fan_rpms: [0, 0, 0, 0],
            current_pwm: [0, 0, 0, 0],
            cmd_seq: 0,
            fan_type: WirelessFanType::SlInf,
            list_index: 0,
            missed_polls,
        }
    }

    #[test]
    fn preserves_available_devices_across_transient_discovery_misses() {
        let previous = vec![test_device(0x01, [0; 6], 0)];
        let mut found = vec![test_device(0x02, [0; 6], 0)];

        merge_missing_discovery_devices(&previous, &mut found, &[0; 6]);

        assert_eq!(found.len(), 2);
        let preserved = found
            .iter()
            .find(|device| device.mac == previous[0].mac)
            .expect("available device should be preserved");
        assert_eq!(preserved.master_mac, [0; 6]);
        assert_eq!(preserved.missed_polls, 1);
    }

    #[test]
    fn drops_devices_after_grace_window_expires() {
        let previous = vec![test_device(0x01, [0; 6], DISCOVERY_DEVICE_GRACE_POLLS)];
        let mut found = vec![test_device(0x02, [0; 6], 0)];

        merge_missing_discovery_devices(&previous, &mut found, &[0; 6]);

        assert_eq!(found.len(), 1);
        assert!(found.iter().all(|device| device.mac != previous[0].mac));
    }
}
