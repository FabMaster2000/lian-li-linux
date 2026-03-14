# Domain Model (Devices and Capabilities)

## Scope
This model is derived from the existing codebase (daemon + shared types + GUI usage).
It is a documentation-only model to guide the future web API.

## Device Identity and Addressing
Devices are addressed by stable string IDs created during discovery.
Common formats in the daemon:
- Wired HID/USB:
  - `hid:<serial>` when a unique serial exists
  - `hid:<vid>:<pid>:<bus-portpath>` when no unique serial exists
    - example: `hid:0416:7372:1-5.3`
- Wireless:
  - `wireless:<mac>`
    - example: `wireless:aa:bb:cc:dd:ee:ff`
- Multi-port fan controllers:
  - ports are exposed as child IDs: `<base_id>:port<index>`
    - example: `hid:0416:7372:1-5.3:port2`

Notes:
- LCD config uses either `serial` or `index` in `LcdConfig`. The daemon matches
  by `serial` (string) when provided; otherwise by index in the current USB list.

## Device
High-level model for API use:
```
Device {
  id: string,             // stable device id (see above)
  name: string,           // human readable name
  family: DeviceFamily,   // low-level family enum
  type: DeviceType,       // higher-level category
  capabilities: Capabilities,
  state: DeviceState      // optional, may be partial
}
```

### DeviceFamily (from `lianli-shared::device_id::DeviceFamily`)
Examples:
- `Ene6k77`, `TlFan`, `TlLcd`, `Galahad2Trinity`, `HydroShiftLcd`, `Galahad2Lcd`
- `Slv3Lcd`, `Slv3Led`, `Tlv2Lcd`, `Tlv2Led`, `SlInf`, `Clv1`
- `HydroShift2Lcd`, `Lancool207`, `UniversalScreen`
- `WirelessTx`, `WirelessRx`, `DisplaySwitcher`

### DeviceType (derived category)
Suggested categories for API/modeling:
- `FanControllerWired`
- `FanControllerWireless`
- `RgbControllerWired`
- `RgbControllerWireless`
- `LcdDevice`
- `AioCooler`
- `BridgeOrUtility` (TX/RX dongle, display switcher)

Mapping is derived from `DeviceInfo` flags (`has_fan`, `has_rgb`, `has_lcd`, `has_pump`) plus family.

## Capabilities
`Capabilities` is a composition of optional feature blocks.
```
Capabilities {
  lighting?: LightingCapability,
  fan?: FanCapability,
  profile?: ProfileCapability,
  sensor?: SensorCapability
}
```

### LightingCapability
Derived from `RgbDeviceCapabilities` plus `DeviceInfo`:
```
LightingCapability {
  supported_modes: [RgbMode],
  zones: [
    {
      zone_index: u8,
      name: string,
      led_count: u16,
      supported_scopes: [RgbScope] // may be empty => only "All"
    }
  ],
  supports_direct: bool,
  supports_mb_rgb_sync: bool,
  supports_direction: bool,
  total_led_count: u16
}
```
Notes:
- Wired devices expose native hardware modes (see `RgbMode`).
- Wireless devices expose `Static` and `Direct` only.
- `supported_scopes` indicates group zones (Top/Bottom/etc.) for certain devices.

### FanCapability
Derived from `DeviceInfo` + `FanDevice` trait:
```
FanCapability {
  fan_count: u8,              // per port if multi-port
  per_fan_control: bool,      // true = per fan, false = per port
  ports: [ { port: u8, fan_count: u8 } ],
  supports_mb_sync: bool,     // hardware PWM sync
  control_mode: "config" | "direct"
}
```
Notes:
- The daemon currently applies fan control from `AppConfig` (config-driven loop).
- `IpcRequest::SetFanSpeed` exists but is not used to drive hardware in the daemon.
- Multi-port devices are modeled as per-port `DeviceInfo` entries with `:portN` IDs.

### ProfileCapability (future)
Profiles are not implemented in the current codebase.
Proposed structure (for Phase 6):
```
ProfileCapability {
  supported: false,
  storage: "none" | "json" | "sqlite"
}
```

### SensorCapability
Sensor data and telemetry are partial and device-specific.
```
SensorCapability {
  telemetry: {
    fan_rpms?: { [device_id]: [u16] },
    coolant_temps?: { [device_id]: f32 }
  },
  lcd_sensor_gauges?: {
    source: "command" | "constant",
    update_interval_ms: u64
  }
}
```
Notes:
- `TelemetrySnapshot` includes `fan_rpms` and `coolant_temps` but coolant temps
  may be empty depending on device support/implementation.
- LCD sensor gauges are rendered from `SensorDescriptor` (command-driven).

## DeviceState (current reality)
The daemon exposes limited live state:
- Fan RPMs: via `GetTelemetry`.
- RGB state: not returned directly; the GUI writes effects and relies on config.
- LCD state: inferred from config and streaming state (`telemetry.streaming_active`).

For the web API, expect to build state by combining:
- `ListDevices` (capabilities)
- `GetTelemetry` (live RPMs, streaming status)
- `GetConfig` (current config as desired state)

## Summary for API Design
- Use `DeviceInfo` as the base list, enrich with `RgbDeviceCapabilities`.
- Treat fan and LCD state as config-driven (write config, then observe via telemetry).
- Represent ports and zones explicitly (ports for fan, zones for RGB).
- Keep IDs stable and string-based; preserve `:portN` suffixes and `wireless:` prefix.
