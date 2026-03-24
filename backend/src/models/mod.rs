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
pub struct DeviceControllerContext {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct DeviceWirelessContext {
    pub transport: String,
    pub channel: Option<u8>,
    pub group_id: Option<String>,
    pub group_label: Option<String>,
    pub binding_state: Option<String>,
    pub master_mac: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeviceHealthState {
    pub level: String,
    pub summary: String,
}

#[derive(Debug, Serialize)]
pub struct DeviceView {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub family: String,
    pub online: bool,
    pub ui_order: u32,
    pub physical_role: String,
    pub capability_summary: String,
    pub current_mode_summary: String,
    pub controller: DeviceControllerContext,
    pub wireless: Option<DeviceWirelessContext>,
    pub health: DeviceHealthState,
    pub capabilities: DeviceCapability,
    pub state: DeviceState,
}

#[derive(Debug, Deserialize)]
pub struct DevicePresentationUpdateRequest {
    pub display_name: String,
    pub ui_order: u32,
    pub physical_role: Option<String>,
    pub controller_label: Option<String>,
    pub cluster_label: Option<String>,
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
    #[serde(default)]
    pub colors: Vec<ColorValue>,
    pub direction: Option<String>,
    pub scope: Option<String>,
    #[serde(default)]
    pub smoothness_ms: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub struct LightingApplyRequest {
    pub target_mode: String,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub device_ids: Vec<String>,
    pub zone_mode: String,
    #[serde(default)]
    pub zone: Option<u8>,
    #[serde(default)]
    pub sync_selected: bool,
    pub effect: String,
    pub brightness: u8,
    #[serde(default)]
    pub speed: Option<u8>,
    #[serde(default)]
    pub color: Option<ColorValue>,
    #[serde(default)]
    pub colors: Vec<ColorValue>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub smoothness_ms: Option<u16>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct LightingLedZoneConfigDocument {
    pub zone: u8,
    #[serde(default)]
    pub led_indexes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightingEffectRouteEntryDocument {
    pub device_id: String,
    pub fan_index: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightingEffectRouteResponse {
    #[serde(default)]
    pub route: Vec<LightingEffectRouteEntryDocument>,
}

#[derive(Debug, Deserialize)]
pub struct LightingEffectRouteSaveRequest {
    #[serde(default)]
    pub route: Vec<LightingEffectRouteEntryDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WirelessDiscoveryRefreshResponse {
    pub refreshed: bool,
    pub device_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WirelessConnectResponse {
    pub device_id: String,
    pub connected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WirelessDisconnectResponse {
    pub device_id: String,
    pub disconnected: bool,
}

#[derive(Debug, Serialize)]
pub struct LightingApplyDeviceState {
    pub device_id: String,
    pub zones: Vec<LightingZoneState>,
}

#[derive(Debug, Serialize)]
pub struct LightingApplySkip {
    pub device_id: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct LightingApplyResponse {
    pub target_mode: String,
    pub zone_mode: String,
    pub requested_device_ids: Vec<String>,
    pub applied_devices: Vec<LightingApplyDeviceState>,
    pub skipped_devices: Vec<LightingApplySkip>,
    pub sync_selected: bool,
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
    pub smoothness_ms: u16,
}

#[derive(Debug, Deserialize)]
pub struct FanManualRequest {
    pub percent: u8,
    pub slot: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct FanCurveUpsertRequest {
    pub name: String,
    pub temperature_source: String,
    #[serde(default)]
    pub points: Vec<FanCurvePointDocument>,
}

#[derive(Debug, Deserialize)]
pub struct FanApplyRequest {
    pub target_mode: String,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub device_ids: Vec<String>,
    pub mode: String,
    #[serde(default)]
    pub percent: Option<u8>,
    #[serde(default)]
    pub curve: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FanStateResponse {
    pub device_id: String,
    pub update_interval_ms: u64,
    pub rpms: Option<Vec<u16>>,
    pub slots: Vec<FanSlotState>,
    pub active_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_curve: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature_source: Option<String>,
    pub mb_sync_enabled: bool,
    pub start_stop_supported: bool,
    pub start_stop_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct FanSlotState {
    pub slot: u8,
    pub mode: String,
    pub percent: Option<u8>,
    pub pwm: Option<u8>,
    pub curve: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FanApplyResponse {
    pub target_mode: String,
    pub mode: String,
    pub requested_device_ids: Vec<String>,
    pub applied_devices: Vec<FanStateResponse>,
    pub skipped_devices: Vec<FanApplySkip>,
}

#[derive(Debug, Serialize)]
pub struct FanApplySkip {
    pub device_id: String,
    pub reason: String,
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
    pub global_led_zones: Vec<LightingLedZoneConfigDocument>,
    #[serde(default)]
    pub fan_led_zones: Vec<LightingFanLedZoneConfigDocument>,
    #[serde(default)]
    pub effect_route: Vec<LightingEffectRouteEntryDocument>,
    #[serde(default)]
    pub devices: Vec<LightingDeviceConfigDocument>,
}

impl Default for LightingConfigDocument {
    fn default() -> Self {
        Self {
            enabled: true,
            openrgb_server: false,
            openrgb_port: 6743,
            global_led_zones: Vec::new(),
            fan_led_zones: Vec::new(),
            effect_route: Vec::new(),
            devices: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightingFanLedZoneConfigDocument {
    pub device_id: String,
    pub fan_index: u8,
    #[serde(default)]
    pub zones: Vec<LightingLedZoneConfigDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightingDeviceConfigDocument {
    pub device_id: String,
    pub motherboard_sync: bool,
    #[serde(default)]
    pub led_zones: Vec<LightingLedZoneConfigDocument>,
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
    #[serde(default)]
    pub smoothness_ms: u16,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanCurveDocument {
    pub name: String,
    pub temperature_source: String,
    #[serde(default)]
    pub points: Vec<FanCurvePointDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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








