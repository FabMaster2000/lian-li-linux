use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct RuntimeResponse {
    pub backend: BackendRuntime,
    pub daemon: DaemonRuntime,
}

#[derive(Debug, Serialize)]
pub struct BackendRuntime {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub config_path: String,
    pub profile_store_path: String,
    pub auth: BackendAuthRuntime,
}

#[derive(Debug, Serialize)]
pub struct BackendAuthRuntime {
    pub enabled: bool,
    pub mode: String,
    pub reload_requires_restart: bool,
    pub basic_username_configured: bool,
    pub basic_password_configured: bool,
    pub token_configured: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_header: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DaemonRuntime {
    pub socket_path: String,
    pub config_path: String,
    pub xdg_runtime_dir: String,
    pub xdg_config_home: String,
}

#[derive(Debug, Serialize)]
pub struct DaemonStatusResponse {
    pub reachable: bool,
    pub socket_path: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeviceCapability {
    pub has_fan: bool,
    pub has_rgb: bool,
    pub has_lcd: bool,
    pub has_pump: bool,
    pub fan_count: Option<u8>,
    pub per_fan_control: Option<bool>,
    pub mb_sync_support: bool,
    pub rgb_zone_count: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct DeviceState {
    pub fan_rpms: Option<Vec<u16>>,
    pub coolant_temp: Option<f32>,
    pub streaming_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct DeviceView {
    pub id: String,
    pub name: String,
    pub family: String,
    pub online: bool,
    pub capabilities: DeviceCapability,
    pub state: DeviceState,
}

#[derive(Debug, Deserialize)]
pub struct ColorValue {
    pub hex: Option<String>,
    pub rgb: Option<RgbColor>,
}

#[derive(Debug, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Deserialize)]
pub struct LightingColorRequest {
    pub zone: Option<u8>,
    pub color: ColorValue,
}

#[derive(Debug, Deserialize)]
pub struct LightingEffectRequest {
    pub zone: Option<u8>,
    pub effect: String,
    pub speed: Option<u8>,
    pub brightness: Option<u8>,
    pub color: Option<ColorValue>,
    pub direction: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LightingBrightnessRequest {
    pub zone: Option<u8>,
    pub percent: u8,
}

#[derive(Debug, Serialize)]
pub struct LightingStateResponse {
    pub device_id: String,
    pub zones: Vec<LightingZoneState>,
}

#[derive(Debug, Serialize)]
pub struct LightingZoneState {
    pub zone: u8,
    pub effect: String,
    pub colors: Vec<String>,
    pub speed: u8,
    pub brightness_percent: u8,
    pub direction: String,
    pub scope: String,
}

#[derive(Debug, Deserialize)]
pub struct FanManualRequest {
    pub percent: u8,
    pub slot: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct FanStateResponse {
    pub device_id: String,
    pub update_interval_ms: u64,
    pub rpms: Option<Vec<u16>>,
    pub slots: Vec<FanSlotState>,
}

#[derive(Debug, Serialize)]
pub struct FanSlotState {
    pub slot: u8,
    pub mode: String,
    pub percent: Option<u8>,
    pub pwm: Option<u8>,
    pub curve: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigDocument {
    pub default_fps: f32,
    pub hid_driver: String,
    #[serde(default)]
    pub lighting: LightingConfigDocument,
    #[serde(default)]
    pub fans: FanConfigDocument,
    #[serde(default)]
    pub lcds: Vec<LcdConfigDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightingConfigDocument {
    pub enabled: bool,
    pub openrgb_server: bool,
    pub openrgb_port: u16,
    #[serde(default)]
    pub devices: Vec<LightingDeviceConfigDocument>,
}

impl Default for LightingConfigDocument {
    fn default() -> Self {
        Self {
            enabled: true,
            openrgb_server: false,
            openrgb_port: 6743,
            devices: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightingDeviceConfigDocument {
    pub device_id: String,
    pub motherboard_sync: bool,
    #[serde(default)]
    pub zones: Vec<LightingZoneConfigDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightingZoneConfigDocument {
    pub zone: u8,
    pub effect: String,
    #[serde(default)]
    pub colors: Vec<String>,
    pub speed: u8,
    pub brightness_percent: u8,
    pub direction: String,
    pub scope: String,
    pub swap_left_right: bool,
    pub swap_top_bottom: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FanConfigDocument {
    pub update_interval_ms: u64,
    #[serde(default)]
    pub curves: Vec<FanCurveDocument>,
    #[serde(default)]
    pub devices: Vec<FanDeviceConfigDocument>,
}

impl Default for FanConfigDocument {
    fn default() -> Self {
        Self {
            update_interval_ms: 1000,
            curves: Vec::new(),
            devices: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FanCurveDocument {
    pub name: String,
    pub temperature_source: String,
    #[serde(default)]
    pub points: Vec<FanCurvePointDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FanCurvePointDocument {
    pub temperature_celsius: f32,
    pub percent: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FanDeviceConfigDocument {
    pub device_id: Option<String>,
    #[serde(default)]
    pub slots: Vec<FanSlotConfigDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FanSlotConfigDocument {
    pub slot: u8,
    pub mode: String,
    pub percent: Option<u8>,
    pub curve: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LcdConfigDocument {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    pub index: Option<usize>,
    pub serial: Option<String>,
    pub media: String,
    pub path: Option<String>,
    pub fps: Option<f32>,
    pub color: Option<String>,
    pub orientation: f32,
    pub sensor: Option<SensorConfigDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SensorConfigDocument {
    pub label: String,
    pub unit: String,
    pub source: SensorSourceDocument,
    pub text_color: String,
    pub background_color: String,
    pub gauge_background_color: String,
    #[serde(default)]
    pub ranges: Vec<SensorRangeDocument>,
    pub update_interval_ms: u64,
    pub gauge_start_angle: f32,
    pub gauge_sweep_angle: f32,
    pub gauge_outer_radius: f32,
    pub gauge_thickness: f32,
    pub bar_corner_radius: f32,
    pub value_font_size: f32,
    pub unit_font_size: f32,
    pub label_font_size: f32,
    pub font_path: Option<String>,
    pub decimal_places: u8,
    pub value_offset: i32,
    pub unit_offset: i32,
    pub label_offset: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SensorSourceDocument {
    Constant { value: f32 },
    Command { command: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SensorRangeDocument {
    pub max: Option<f32>,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileDocument {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub targets: ProfileTargetsDocument,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting: Option<ProfileLightingDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fans: Option<ProfileFanDocument>,
    pub metadata: ProfileMetadataDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileUpsertDocument {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub targets: ProfileTargetsDocument,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting: Option<ProfileLightingDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fans: Option<ProfileFanDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileTargetsDocument {
    pub mode: String,
    #[serde(default)]
    pub device_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileLightingDocument {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brightness_percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileFanDocument {
    pub enabled: bool,
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileMetadataDocument {
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileApplyResponse {
    pub profile_id: String,
    pub profile_name: String,
    pub transaction_mode: String,
    pub rollback_supported: bool,
    pub applied_lighting_device_ids: Vec<String>,
    pub applied_fan_device_ids: Vec<String>,
    pub skipped_devices: Vec<ProfileApplySkip>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileApplySkip {
    pub device_id: String,
    pub section: String,
    pub reason: String,
}
