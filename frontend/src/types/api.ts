export type ApiErrorCode =
  | "BAD_REQUEST"
  | "UNAUTHORIZED"
  | "NOT_FOUND"
  | "DAEMON_ERROR"
  | "INTERNAL_ERROR";

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

export type ApiErrorResponse = {
  error: {
    code: ApiErrorCode;
    message: string;
    details: Record<string, JsonValue>;
  };
};

export type HealthResponse = {
  status: string;
};

export type VersionResponse = {
  version: string;
};

export type BackendAuthRuntime = {
  enabled: boolean;
  mode: string;
  reload_requires_restart: boolean;
  basic_username_configured: boolean;
  basic_password_configured: boolean;
  token_configured: boolean;
  proxy_header: string | null;
};

export type RuntimeResponse = {
  backend: {
    host: string;
    port: number;
    log_level: string;
    config_path: string;
    profile_store_path: string;
    auth: BackendAuthRuntime;
  };
  daemon: {
    socket_path: string;
    config_path: string;
    xdg_runtime_dir: string;
    xdg_config_home: string;
  };
};

export type DaemonStatusResponse = {
  reachable: boolean;
  socket_path: string;
  error: string | null;
};

export type DeviceCapability = {
  has_fan: boolean;
  has_rgb: boolean;
  has_lcd: boolean;
  has_pump: boolean;
  fan_count: number | null;
  per_fan_control: boolean | null;
  mb_sync_support: boolean;
  rgb_zone_count: number | null;
};

export type DeviceState = {
  fan_rpms: number[] | null;
  coolant_temp: number | null;
  streaming_active: boolean | null;
};

export type DeviceControllerContext = {
  id: string;
  label: string;
  kind: string;
};

export type DeviceWirelessContext = {
  transport: string;
  channel: number | null;
  group_id: string | null;
  group_label: string | null;
  binding_state: "connected" | "available" | "foreign" | null;
  master_mac: string | null;
};

export type DeviceHealthState = {
  level: string;
  summary: string;
};

export type DeviceView = {
  id: string;
  name: string;
  display_name: string;
  family: string;
  online: boolean;
  ui_order: number;
  physical_role: string;
  capability_summary: string;
  current_mode_summary: string;
  controller: DeviceControllerContext;
  wireless: DeviceWirelessContext | null;
  health: DeviceHealthState;
  capabilities: DeviceCapability;
  state: DeviceState;
};

export type DevicePresentationUpdateRequest = {
  display_name: string;
  ui_order: number;
  physical_role?: string | null;
  controller_label?: string | null;
  cluster_label?: string | null;
};

export type WirelessDisconnectResponse = {
  device_id: string;
  disconnected: boolean;
};

export type WirelessConnectResponse = {
  device_id: string;
  connected: boolean;
};

export type WirelessDiscoveryRefreshResponse = {
  refreshed: boolean;
  device_count: number;
};

export type RgbColor = {
  r: number;
  g: number;
  b: number;
};

export type ColorValue = {
  hex?: string;
  rgb?: RgbColor;
};

export type LightingColorRequest = {
  zone?: number;
  color: ColorValue;
};

export type LightingEffectRequest = {
  zone?: number;
  effect: string;
  speed?: number;
  brightness?: number;
  color?: ColorValue;
  colors?: ColorValue[];
  direction?: string;
  scope?: string;
  smoothness_ms?: number;
};

export type LightingApplyRequest = {
  target_mode: "single" | "selected" | "all" | "route";
  device_id?: string | null;
  device_ids?: string[];
  zone_mode: "active" | "all_zones";
  zone?: number | null;
  sync_selected?: boolean;
  effect: string;
  brightness: number;
  speed?: number;
  color?: ColorValue | null;
  colors?: ColorValue[];
  direction?: string | null;
  scope?: string | null;
  smoothness_ms?: number;
};

export type LightingBrightnessRequest = {
  zone?: number;
  percent: number;
};

export type LightingZoneState = {
  zone: number;
  effect: string;
  colors: string[];
  speed: number;
  brightness_percent: number;
  direction: string;
  scope: string;
  smoothness_ms: number;
};

export type LightingStateResponse = {
  device_id: string;
  zones: LightingZoneState[];
};

export type LightingLedZoneConfigDocument = {
  zone: number;
  led_indexes: number[];
};

export type LightingZoneLayoutResponse = {
  device_id: string;
  device_name: string;
  fan_count: number;
  leds_per_fan: number;
  suggested_zone_count: number;
  zones: LightingLedZoneConfigDocument[];
};

export type LightingZoneLayoutSaveRequest = {
  fan_index: number;
  zones: LightingLedZoneConfigDocument[];
};

export type LightingLedProbeRequest = {
  fan_index: number;
  led_index: number;
  color: ColorValue;
};

export type LightingLedProbeResponse = {
  device_id: string;
  fan_index: number;
  led_index: number;
  applied: boolean;
};

export type LightingZoneLayoutRestoreResponse = {
  device_id: string;
  restored: boolean;
};

export type LightingApplyDeviceState = {
  device_id: string;
  zones: LightingZoneState[];
};

export type LightingApplySkip = {
  device_id: string;
  reason: string;
};

export type LightingApplyResponse = {
  target_mode: string;
  zone_mode: string;
  requested_device_ids: string[];
  applied_devices: LightingApplyDeviceState[];
  skipped_devices: LightingApplySkip[];
  sync_selected: boolean;
};

export type FanManualRequest = {
  percent: number;
  slot?: number;
};

export type FanCurveUpsertRequest = {
  name: string;
  temperature_source: string;
  points: FanCurvePointDocument[];
};

export type FanTemperatureComponent = {
  key: string;
  label: string;
  kind: string;
  available: boolean;
  celsius: number | null;
  note?: string | null;
};

export type FanTemperaturePreview = {
  source: string;
  display_name: string;
  available: boolean;
  celsius: number | null;
  components: FanTemperatureComponent[];
  note?: string | null;
};

export type FanApplyRequest = {
  target_mode: "single" | "selected" | "all";
  device_id?: string | null;
  device_ids?: string[];
  mode: "manual" | "curve" | "mb_sync" | "start_stop";
  percent?: number | null;
  curve?: string | null;
};

export type FanSlotState = {
  slot: number;
  mode: string;
  percent: number | null;
  pwm: number | null;
  curve: string | null;
};

export type FanStateResponse = {
  device_id: string;
  update_interval_ms: number;
  rpms: number[] | null;
  slots: FanSlotState[];
  active_mode?: string;
  active_curve?: string | null;
  temperature_source?: string | null;
  mb_sync_enabled?: boolean;
  start_stop_supported?: boolean;
  start_stop_enabled?: boolean;
};

export type FanApplySkip = {
  device_id: string;
  reason: string;
};

export type FanApplyResponse = {
  target_mode: string;
  mode: string;
  requested_device_ids: string[];
  applied_devices: FanStateResponse[];
  skipped_devices: FanApplySkip[];
};

export type LightingConfigDocument = {
  enabled: boolean;
  openrgb_server: boolean;
  openrgb_port: number;
  global_led_zones: LightingLedZoneConfigDocument[];
  fan_led_zones: LightingFanLedZoneConfigDocument[];
  devices: LightingDeviceConfigDocument[];
};

export type LightingFanLedZoneConfigDocument = {
  device_id: string;
  fan_index: number;
  zones: LightingLedZoneConfigDocument[];
};

export type LightingDeviceConfigDocument = {
  device_id: string;
  motherboard_sync: boolean;
  led_zones: LightingLedZoneConfigDocument[];
  zones: LightingZoneConfigDocument[];
};

export type LightingZoneConfigDocument = {
  zone: number;
  effect: string;
  colors: string[];
  speed: number;
  brightness_percent: number;
  direction: string;
  scope: string;
  swap_left_right: boolean;
  swap_top_bottom: boolean;
  smoothness_ms: number;
};

export type FanCurvePointDocument = {
  temperature_celsius: number;
  percent: number;
};

export type FanCurveDocument = {
  name: string;
  temperature_source: string;
  points: FanCurvePointDocument[];
};

export type FanSlotConfigDocument = {
  slot: number;
  mode: string;
  percent: number | null;
  curve: string | null;
};

export type FanDeviceConfigDocument = {
  device_id: string | null;
  slots: FanSlotConfigDocument[];
};

export type FanConfigDocument = {
  update_interval_ms: number;
  curves: FanCurveDocument[];
  devices: FanDeviceConfigDocument[];
};

export type SensorSourceDocument =
  | {
      kind: "constant";
      value: number;
    }
  | {
      kind: "command";
      command: string;
    };

export type SensorRangeDocument = {
  max: number | null;
  color: string;
};

export type SensorConfigDocument = {
  label: string;
  unit: string;
  source: SensorSourceDocument;
  text_color: string;
  background_color: string;
  gauge_background_color: string;
  ranges: SensorRangeDocument[];
  update_interval_ms: number;
  gauge_start_angle: number;
  gauge_sweep_angle: number;
  gauge_outer_radius: number;
  gauge_thickness: number;
  bar_corner_radius: number;
  value_font_size: number;
  unit_font_size: number;
  label_font_size: number;
  font_path: string | null;
  decimal_places: number;
  value_offset: number;
  unit_offset: number;
  label_offset: number;
};

export type LcdConfigDocument = {
  device_id: string | null;
  index: number | null;
  serial: string | null;
  media: string;
  path: string | null;
  fps: number | null;
  color: string | null;
  orientation: number;
  sensor: SensorConfigDocument | null;
};

export type ConfigDocument = {
  default_fps: number;
  hid_driver: string;
  lighting: LightingConfigDocument;
  fans: FanConfigDocument;
  lcds: LcdConfigDocument[];
};

export type ProfileTargetsDocument = {
  mode: string;
  device_ids: string[];
};

export type ProfileLightingDocument = {
  enabled: boolean;
  color?: string;
  effect?: string;
  brightness_percent?: number;
  speed?: number;
  direction?: string;
  scope?: string;
};

export type ProfileFanDocument = {
  enabled: boolean;
  mode: string;
  percent?: number;
};

export type ProfileMetadataDocument = {
  created_at: string;
  updated_at: string;
};

export type ProfileDocument = {
  id: string;
  name: string;
  description?: string;
  targets: ProfileTargetsDocument;
  lighting?: ProfileLightingDocument;
  fans?: ProfileFanDocument;
  metadata: ProfileMetadataDocument;
};

export type ProfileUpsertDocument = {
  id: string;
  name: string;
  description?: string;
  targets: ProfileTargetsDocument;
  lighting?: ProfileLightingDocument;
  fans?: ProfileFanDocument;
};

export type ProfileApplySkip = {
  device_id: string;
  section: string;
  reason: string;
};

export type ProfileApplyResponse = {
  profile_id: string;
  profile_name: string;
  transaction_mode: string;
  rollback_supported: boolean;
  applied_lighting_device_ids: string[];
  applied_fan_device_ids: string[];
  skipped_devices: ProfileApplySkip[];
};

export type BackendEventType =
  | "daemon.connected"
  | "daemon.disconnected"
  | "device.updated"
  | "fan.changed"
  | "lighting.changed"
  | "config.changed"
  | "openrgb.changed"
  | (string & {});

export type BackendEventEnvelope<TData extends JsonValue = JsonValue> = {
  type: BackendEventType;
  timestamp: string;
  source: string;
  device_id: string | null;
  data: TData;
};



