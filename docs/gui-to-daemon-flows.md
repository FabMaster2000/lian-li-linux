# GUI -> Daemon IPC Flows

This document maps UI actions in `lianli-gui` to IPC requests sent to the daemon.
It focuses on request sequencing, responses, and follow-up actions.

## Device / Telemetry Loading


| Aktion                       | GUI-Funktion                                                    | Request                                              | Response          | Folgeaktionen                                                                                          |
| ---------------------------- | --------------------------------------------------------------- | ---------------------------------------------------- | ----------------- | ------------------------------------------------------------------------------------------------------ |
| App-Start: initial load      | `backend::run_backend`                                          | `Ping` -> `ListDevices` -> `GetTelemetry`            | `IpcResponse::Ok` | UI models updated with device list + telemetry (poll_daemon).                                          |
| App-Start: config + RGB caps | `backend::run_backend`                                          | `GetConfig` -> `GetRgbCapabilities` -> `ListDevices` | `IpcResponse::Ok` | UI models updated with config, RGB caps, devices (load_config).                                        |
| Refresh-Button               | `window.on_refresh_devices` -> `BackendCommand::RefreshDevices` | `Ping` -> `ListDevices` -> `GetTelemetry`            | `IpcResponse::Ok` | UI models updated (poll_daemon).                                                                       |
| Periodic poll (~2s)          | `backend::run_backend`                                          | `Ping` -> `ListDevices` -> `GetTelemetry`            | `IpcResponse::Ok` | UI updated; on reconnect or device count change -> `GetConfig` + `GetRgbCapabilities` + `ListDevices`. |


## Config Save / Reload


| Aktion                                                            | GUI-Funktion                                                                                                               | Request                                                                          | Response                     | Folgeaktionen                                                                           |
| ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ---------------------------- | --------------------------------------------------------------------------------------- |
| Save Config                                                       | `window.on_save_config` -> `BackendCommand::SaveConfig`                                                                    | Flush pending `SetRgbEffect` / `SetFanDirection`, then `SetConfig { AppConfig }` | `IpcResponse::Ok` or `Error` | On Ok: UI marks clean. Then reloads `GetConfig` + `GetRgbCapabilities` + `ListDevices`. |
| Toggle OpenRGB                                                    | `window.on_toggle_openrgb`                                                                                                 | `SetConfig { AppConfig }`                                                        | `IpcResponse::Ok` or `Error` | Same SaveConfig flow; daemon may start/stop OpenRGB server.                             |
| Set default FPS / OpenRGB port / HID driver / fan update interval | `window.on_set_default_fps`, `window.on_set_openrgb_port`, `window.on_set_hid_driver`, `window.on_fan_set_update_interval` | none (in-memory config only)                                                     | n/a                          | Requires Save Config to send `SetConfig`.                                               |
| Fan curves / fan slots / LCD fields                               | various `window.on_fan_*` / `window.on_*lcd*`                                                                              | none (in-memory config only)                                                     | n/a                          | Requires Save Config to send `SetConfig`.                                               |


## RGB Actions


| Aktion                            | GUI-Funktion                                                    | Request                                                                 | Response                                   | Folgeaktionen                                               |
| --------------------------------- | --------------------------------------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------ | ----------------------------------------------------------- |
| Set RGB mode                      | `window.on_rgb_set_mode` -> `send_rgb_effect`                   | `SetRgbEffect { device_id, zone, effect }` (may broadcast to all zones) | `IpcResponse::Ok` or `Error` (logged only) | UI updated in-place; config marked dirty.                   |
| Set RGB speed                     | `window.on_rgb_set_speed` -> `send_rgb_effect`                  | `SetRgbEffect`                                                          | `IpcResponse::Ok` or `Error`               | UI updated; config dirty.                                   |
| Set RGB brightness                | `window.on_rgb_set_brightness` -> `send_rgb_effect`             | `SetRgbEffect`                                                          | `IpcResponse::Ok` or `Error`               | UI updated; config dirty.                                   |
| Set RGB direction                 | `window.on_rgb_set_direction` -> `send_rgb_effect`              | `SetRgbEffect`                                                          | `IpcResponse::Ok` or `Error`               | UI updated; config dirty.                                   |
| Set RGB scope                     | `window.on_rgb_set_scope` -> `send_rgb_effect`                  | `SetRgbEffect`                                                          | `IpcResponse::Ok` or `Error`               | UI updated; config dirty.                                   |
| Set RGB color                     | `window.on_rgb_set_color` -> `send_rgb_effect`                  | `SetRgbEffect`                                                          | `IpcResponse::Ok` or `Error`               | UI updated; config dirty.                                   |
| Add/Remove RGB color slot         | `window.on_rgb_add_color` / `window.on_rgb_remove_color`        | `SetRgbEffect`                                                          | `IpcResponse::Ok` or `Error`               | UI updated; config dirty.                                   |
| Apply zone 0 effect to all zones  | `window.on_rgb_apply_to_all`                                    | Multiple `SetRgbEffect` (per zone)                                      | `IpcResponse::Ok` or `Error`               | No additional UI changes.                                   |
| Toggle MB RGB Sync                | `window.on_rgb_toggle_mb_sync`                                  | `SetMbRgbSync { device_id, enabled }`                                   | `IpcResponse::Ok` or `Error`               | Config updated for sibling ports; UI updated; config dirty. |
| Toggle fan direction swap LR / TB | `window.on_rgb_toggle_swap_lr` / `window.on_rgb_toggle_swap_tb` | `SetFanDirection { device_id, zone, swap_lr, swap_tb }`                 | `IpcResponse::Ok` or `Error`               | UI updated; config dirty.                                   |


Notes:

- `SetRgbEffect` and `SetFanDirection` are debounced in `backend.rs` (50ms). Latest request per device/zone wins.
- GUI does not use `SetRgbDirect`; OpenRGB uses that path via the daemon.

## Fan Actions (Config-Driven)


| Aktion                                           | GUI-Funktion                                                            | Request | Response | Folgeaktionen                                   |
| ------------------------------------------------ | ----------------------------------------------------------------------- | ------- | -------- | ----------------------------------------------- |
| Add/Remove/rename curve                          | `window.on_fan_add_curve`, `on_fan_remove_curve`, `on_fan_rename_curve` | none    | n/a      | Config updated in memory; Save Config required. |
| Change curve points                              | `window.on_fan_point_*`                                                 | none    | n/a      | Config updated; Save Config required.           |
| Assign fan slot speed (curve, MB sync, constant) | `window.on_fan_set_slot_speed`                                          | none    | n/a      | Config updated; Save Config required.           |
| Set fan slot PWM percent                         | `window.on_fan_set_slot_pwm`                                            | none    | n/a      | Config updated; Save Config required.           |


## LCD Actions (Config-Driven)


| Aktion                                                       | GUI-Funktion                                | Request | Response | Folgeaktionen                         |
| ------------------------------------------------------------ | ------------------------------------------- | ------- | -------- | ------------------------------------- |
| Add/Remove LCD entry                                         | `window.on_add_lcd`, `window.on_remove_lcd` | none    | n/a      | Config updated; Save Config required. |
| Update LCD fields (device, media, sensor, colors, fps, etc.) | `window.on_update_lcd_field`                | none    | n/a      | Config updated; Save Config required. |
| Pick LCD media file                                          | `window.on_pick_lcd_file`                   | none    | n/a      | Config updated; Save Config required. |


## Profiles

- No profile UI exists in `lianli-gui` at this time.
- No profile-related IPC calls are made.

