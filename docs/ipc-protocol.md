# IPC Protocol (Daemon <-> Client)

## Scope
This document describes IPC types defined in `crates/lianli-shared/src/ipc.rs` and how they are serialized.
It is the basis for a future headless web backend that reuses the daemon IPC.

## Serialization
- Format: JSON via `serde` / `serde_json`.
- Requests: `IpcRequest` uses `#[serde(tag = "method", content = "params")]`.
  - Example shape:
    - `{ "method": "ListDevices" }`
    - `{ "method": "SetRgbEffect", "params": { ... } }`
- Responses: `IpcResponse` uses `#[serde(tag = "status")]` with `"ok"` / `"error"`.
  - Ok: `{ "status": "ok", "data": <any JSON> }`
  - Error: `{ "status": "error", "message": "..." }`
- Events: `IpcEvent` uses `#[serde(tag = "event", content = "data")]`.

## Framing / Transport
- Transport: Unix Domain Socket (see daemon IPC server).
- Framing: newline-delimited JSON (1 request per line, 1 response per line).
- The server accepts multiple lines per connection, but the current GUI client sends
  one request and closes the write side after newline.

## Versioning
- No explicit protocol version field exists in IPC messages.
- Backwards/forwards compatibility relies on serde defaults and tolerant parsing.

## Requests (`IpcRequest`)

### Device and Telemetry
- `ListDevices`
  - Params: none
  - Response data: `Vec<DeviceInfo>`
- `GetTelemetry`
  - Params: none
  - Response data: `TelemetrySnapshot`

### Configuration
- `GetConfig`
  - Params: none
  - Response data: `AppConfig`
- `SetConfig { config: AppConfig }`
  - Replaces full config; daemon writes to disk and reloads.
- `SetFanConfig { config: FanConfig }`
- `SetRgbConfig { config: RgbAppConfig }`
- `SetLcdMedia { device_id: String, config: LcdConfig }`

### Fan control
- `SetFanSpeed { device_index: u8, fan_pwm: [u8; 4] }`
  - Legacy style (device index). Current daemon path logs only.
- `SetFanDirection { device_id: String, zone: u8, swap_lr: bool, swap_tb: bool }`

### RGB control
- `GetRgbCapabilities`
  - Response data: `Vec<RgbDeviceCapabilities>`
- `SetRgbEffect { device_id: String, zone: u8, effect: RgbEffect }`
- `SetRgbDirect { device_id: String, zone: u8, colors: Vec<[u8; 3]> }`
- `SetMbRgbSync { device_id: String, enabled: bool }`

### Other
- `Ping`
  - Response data: "pong"
- `Subscribe`
  - Intended for event streaming; currently not implemented in daemon (returns error).

## Responses (`IpcResponse`)
- `Ok { data: serde_json::Value }`
  - Any JSON payload, depending on the request.
- `Error { message: String }`

## Events (`IpcEvent`)
Defined but currently not streamed via IPC:
- `DeviceAttached { device_id, family, name }`
- `DeviceDetached { device_id }`
- `ConfigChanged`
- `FanSpeedUpdate { device_index, rpms: Vec<u16> }`

## Shared Data Models

### `DeviceInfo`
Fields:
- `device_id: String`
- `family: DeviceFamily`
- `name: String`
- `serial: Option<String>`
- `has_lcd: bool`
- `has_fan: bool`
- `has_pump: bool`
- `has_rgb: bool`
- `fan_count: Option<u8>`
- `per_fan_control: Option<bool>`
- `mb_sync_support: bool`
- `rgb_zone_count: Option<u8>`
- `screen_width: Option<u32>`
- `screen_height: Option<u32>`

### `TelemetrySnapshot`
Fields:
- `fan_rpms: HashMap<String, Vec<u16>>` (keyed by device_id)
- `coolant_temps: HashMap<String, f32>` (keyed by device_id)
- `streaming_active: bool`
- `openrgb_status: OpenRgbServerStatus`

### `OpenRgbServerStatus`
Fields:
- `enabled: bool`
- `running: bool`
- `port: Option<u16>`
- `error: Option<String>`

## Notes for Web-Backend Mapping
- IPC provides full device list, telemetry, and RGB capabilities already.
- `GetTelemetry` is the current polling path (no event stream yet).
- `SetRgbEffect` / `SetRgbDirect` are the primary control methods for RGB.
- Fan control in IPC is config-driven; `SetFanSpeed` is present but not used in daemon.
