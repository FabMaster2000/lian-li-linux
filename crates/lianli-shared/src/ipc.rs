use crate::config::{AppConfig, LcdConfig};
use crate::device_id::DeviceFamily;
use crate::fan::FanConfig;
use crate::rgb::{RgbAppConfig, RgbEffect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Requests from GUI to daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum IpcRequest {
    ListDevices,
    GetConfig,
    RefreshWirelessDiscovery,
    BindWirelessDevice {
        device_id: String,
    },
    UnbindWirelessDevice {
        device_id: String,
    },
    /// Replace the entire config (daemon writes to disk + reloads).
    SetConfig {
        config: AppConfig,
    },
    SetLcdMedia {
        device_id: String,
        config: LcdConfig,
    },
    SetFanSpeed {
        device_index: u8,
        fan_pwm: [u8; 4],
    },
    SetFanConfig {
        config: FanConfig,
    },
    PreviewFanTemperature {
        source: String,
    },
    GetTelemetry,
    /// Get RGB capabilities for all devices.
    GetRgbCapabilities,
    /// Set RGB effect for a specific device zone.
    SetRgbEffect {
        device_id: String,
        zone: u8,
        effect: RgbEffect,
    },
    /// Set per-LED colors directly (used by OpenRGB integration).
    SetRgbDirect {
        device_id: String,
        zone: u8,
        /// RGB triplets, one per LED.
        colors: Vec<[u8; 3]>,
    },
    /// Light a single physical LED on a selected fan for mapping / probing.
    ProbeRgbLed {
        device_id: String,
        fan_index: u8,
        led_index: u16,
        color: [u8; 3],
    },
    /// Enable/disable motherboard ARGB sync for a device.
    SetMbRgbSync {
        device_id: String,
        enabled: bool,
    },
    /// Set fan direction (swap LR/TB) for a device zone.
    SetFanDirection {
        device_id: String,
        zone: u8,
        swap_lr: bool,
        swap_tb: bool,
    },
    /// Update the RGB configuration section.
    SetRgbConfig {
        config: RgbAppConfig,
    },
    Subscribe,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanTemperaturePreview {
    pub source: String,
    pub display_name: String,
    pub available: bool,
    pub celsius: Option<f32>,
    #[serde(default)]
    pub components: Vec<FanTemperatureComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanTemperatureComponent {
    pub key: String,
    pub label: String,
    pub kind: String,
    pub available: bool,
    pub celsius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Responses from daemon to GUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum IpcResponse {
    #[serde(rename = "ok")]
    Ok { data: serde_json::Value },
    #[serde(rename = "error")]
    Error { message: String },
}

impl IpcResponse {
    pub fn ok(data: impl Serialize) -> Self {
        Self::Ok {
            data: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self::Error {
            message: msg.into(),
        }
    }
}

/// Event notifications pushed from daemon to subscribed clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum IpcEvent {
    DeviceAttached {
        device_id: String,
        family: DeviceFamily,
        name: String,
    },
    DeviceDetached {
        device_id: String,
    },
    ConfigChanged,
    FanSpeedUpdate {
        device_index: u8,
        rpms: Vec<u16>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WirelessBindingState {
    Connected,
    Available,
    Foreign,
}

/// Info about a connected device, returned by ListDevices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub family: DeviceFamily,
    pub name: String,
    pub serial: Option<String>,
    pub wireless_channel: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wireless_missed_polls: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wireless_master_mac: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wireless_binding_state: Option<WirelessBindingState>,
    pub has_lcd: bool,
    pub has_fan: bool,
    pub has_pump: bool,
    pub has_rgb: bool,
    pub fan_count: Option<u8>,
    pub per_fan_control: Option<bool>,
    pub mb_sync_support: bool,
    pub rgb_zone_count: Option<u8>,
    pub screen_width: Option<u32>,
    pub screen_height: Option<u32>,
}

/// Status of the OpenRGB SDK server.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenRgbServerStatus {
    /// Whether the server is enabled in config.
    pub enabled: bool,
    /// Whether the server is currently listening for connections.
    pub running: bool,
    /// The actual port the server bound to (may differ from configured if port was in use).
    pub port: Option<u16>,
    /// Error message if the server failed to start.
    pub error: Option<String>,
}

/// Snapshot of live telemetry data, returned by GetTelemetry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    /// Fan RPMs keyed by device_id.
    pub fan_rpms: HashMap<String, Vec<u16>>,
    /// Coolant temperatures keyed by device_id.
    pub coolant_temps: HashMap<String, f32>,
    /// Whether the daemon is actively streaming frames to LCD devices.
    pub streaming_active: bool,
    /// OpenRGB SDK server status.
    #[serde(default)]
    pub openrgb_status: OpenRgbServerStatus,
}
