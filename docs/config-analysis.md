# Config Analysis

## Scope
This document describes how lianli-daemon stores and loads configuration, which parts persist RGB, fan, and LCD settings, and how configuration changes are applied at runtime.

## Storage Location and Format
- Default path: `$XDG_CONFIG_HOME/lianli/config.json`.
- Fallback path: `$HOME/.config/lianli/config.json` when `XDG_CONFIG_HOME` is not set.
- Override: `lianli-daemon --config /path/to/config.json`.
- Format: JSON (pretty-printed on write).

## Persistence Flow
- On startup, the daemon checks the config path. If the file does not exist, it creates a default JSON config.
- The daemon then loads the config via `AppConfig::load`, validates it, and keeps it in memory.
- IPC config writes (`SetConfig`, `SetFanConfig`, `SetRgbConfig`, `SetLcdMedia`) overwrite the JSON file and set a reload flag.
- The main loop detects the reload flag and re-loads the config.
- If the `hid_driver` changes, the daemon requests a restart so the new backend can be applied.

## AppConfig Schema (Top Level)
Fields in `lianli-shared/src/config.rs`:
- `default_fps` (f32, default 30.0)
- `hid_driver` ("hidapi" or "rusb", default "hidapi")
- `lcds` (array of LCD entries, with `devices` accepted as a legacy alias)
- `fan_curves` (array of named fan curves)
- `fans` (fan control configuration, optional)
- `rgb` (RGB configuration, optional)

## LCD Configuration
Each LCD entry is an `LcdConfig`:
- Identity: either `index` or `serial` must be set.
- Media type: `image`, `video`, `gif`, `color`, or `sensor`.
- Media path required for `image`, `video`, `gif`.
- RGB required for `color`.
- Sensor settings required for `sensor`.
- Optional per-device `fps`, `orientation`, and `sensor` config.

Load-time normalization and validation:
- Relative paths for media and sensor fonts are resolved relative to the config directory.
- Orientation is normalized to the nearest 90 degrees.
- Invalid LCD entries are skipped with warnings (non-fatal).

## Fan Configuration
Defined in `lianli-shared/src/fan.rs`:
- `FanConfig`:
  - `speeds`: array of fan groups.
  - `update_interval_ms`: default 1000.
- `FanGroup`:
  - Optional `device_id` to target a specific device. If missing, groups are matched by discovery order.
  - `speeds`: array of 4 `FanSpeed` entries.
- `FanSpeed`:
  - `Constant(u8)` fixed PWM value.
  - `Curve(String)` references a named curve in `fan_curves`.
  - Reserved curve name `__mb_sync__` enables motherboard RPM sync.
- Backward compatibility: the config loader accepts the legacy `[[FanSpeed; 4]]` array-of-arrays format.

## RGB Configuration
Defined in `lianli-shared/src/rgb.rs`:
- `RgbAppConfig`:
  - `enabled` (default true)
  - `openrgb_server` (default false)
  - `openrgb_port` (default 6743)
  - `devices`: list of per-device configs
- `RgbDeviceConfig`:
  - `device_id`
  - `mb_rgb_sync` (use motherboard ARGB instead of software effects)
  - `zones`: list of `RgbZoneConfig`
- `RgbZoneConfig`:
  - `zone_index`
  - `effect` (mode, colors, speed 0-4, brightness 0-4, direction, scope)
  - `swap_lr`, `swap_tb` for fan orientation

Persistence note:
- `SetRgbConfig` and `SetConfig` write RGB settings to disk.
- `SetRgbEffect` and `SetRgbDirect` are runtime actions and do not persist to the JSON file.

## Runtime Usage
- Fan control: `fan_controller` reads `fans` and `fan_curves` on load and applies them continuously.
- RGB control: `rgb_controller` applies `rgb` on load unless OpenRGB is active.
- OpenRGB server: enabled and configured via `rgb.openrgb_server` and `rgb.openrgb_port`.
- LCD streaming: uses `lcds` and `default_fps` to prepare and stream assets.

## Validation and Error Handling
- `AppConfig::load` validates values and returns warnings for recoverable issues.
- If config parsing or validation fails, the daemon logs warnings and retains the previous in-memory config (or none on first boot).
- Missing config file results in a default config being created.
