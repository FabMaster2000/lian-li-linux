# API (Backend)

## Base URL
- Default: `http://<host>:9000`
- Primary backend config source: `docs/backend-config.md`
- Default backend config file: `$XDG_CONFIG_HOME/lianli/backend.json`
- Environment overrides still exist for:
  - `LIANLI_BACKEND_HOST`
  - `LIANLI_BACKEND_PORT`
  - `LIANLI_DAEMON_SOCKET`
  - `LIANLI_BACKEND_LOG_LEVEL`
  - `LIANLI_DAEMON_CONFIG`
  - `LIANLI_BACKEND_PROFILE_STORE_PATH`
  - `LIANLI_BACKEND_AUTH_*`
- Full backend config reference: `docs/backend-config.md`

## Error Format
All errors use:
```json
{
  "error": {
    "code": "BAD_REQUEST | UNAUTHORIZED | NOT_FOUND | DAEMON_ERROR | INTERNAL_ERROR",
    "message": "...",
    "details": {}
  }
}
```

## Authentication
- Optional and static at backend startup
- Configured through `backend.json` or env overrides
- Changing auth requires a backend restart
- When auth is enabled:
  - `GET /api/health` stays public
  - `GET /api/version` stays public
  - all other `/api/*` routes, including `/api/ws`, require auth

## Endpoints

### GET /api/health
Health check (process alive).

Response:
```json
{ "status": "ok" }
```

### GET /api/version
Backend version.

Response:
```json
{ "version": "0.1.0" }
```

### GET /api/runtime
Runtime information about backend + daemon socket/config paths.

Response:
```json
{
  "backend": {
    "host": "0.0.0.0",
    "port": 9000,
    "log_level": "info",
    "config_path": "/home/user/.config/lianli/backend.json",
    "profile_store_path": "/home/user/.config/lianli/profiles.json",
    "auth": {
      "enabled": false,
      "mode": "none",
      "reload_requires_restart": true,
      "basic_username_configured": false,
      "basic_password_configured": false,
      "token_configured": false
    }
  },
  "daemon": {
    "socket_path": "/tmp/lianli-daemon.sock",
    "config_path": "/home/user/.config/lianli/config.json",
    "xdg_runtime_dir": "/tmp",
    "xdg_config_home": "/home/user/.config"
  }
}
```

Notes:
- Auth secrets are never returned by `/api/runtime`.
- `backend.config_path` points to the backend config file used at startup.
- `auth.reload_requires_restart` is always `true`.

### GET /api/ws
Upgrade to a WebSocket connection that streams backend events as JSON text frames.

Connection:
- URL: `ws://<host>:9000/api/ws`
- Method: `GET`
- transport: WebSocket

Event frame example:
```json
{
  "type": "lighting.changed",
  "timestamp": "2026-03-14T10:15:30Z",
  "source": "api",
  "device_id": "wireless:02:11:22:33:44:55",
  "data": {
    "reason": "color_set",
    "zone": 0,
    "color": "#abcdef"
  }
}
```

Notes:
- The backend currently emits events from two sources:
  - `poll`: snapshot diffing of daemon state
  - `api`: successful backend write operations
- The server sends WebSocket ping frames as keepalive heartbeats.
- Client text or binary messages are ignored.
- Current event types include:
  - `daemon.connected`
  - `daemon.disconnected`
  - `device.updated`
  - `fan.changed`
  - `lighting.changed`
  - `config.changed`
  - `openrgb.changed`
- The event envelope itself is documented in `docs/event-model.md`.

## Config

### GET /api/config
Return a web-friendly view of the daemon config.

Response:
```json
{
  "default_fps": 30.0,
  "hid_driver": "hidapi",
  "lighting": {
    "enabled": true,
    "openrgb_server": false,
    "openrgb_port": 6743,
    "devices": [
      {
        "device_id": "hid:0416:7372:1-5.3",
        "motherboard_sync": false,
        "zones": [
          {
            "zone": 0,
            "effect": "Static",
            "colors": ["#ff0000"],
            "speed": 2,
            "brightness_percent": 75,
            "direction": "Clockwise",
            "scope": "All",
            "swap_left_right": false,
            "swap_top_bottom": false
          }
        ]
      }
    ]
  },
  "fans": {
    "update_interval_ms": 1000,
    "curves": [
      {
        "name": "silent",
        "temperature_source": "sensors | awk '/Package id 0/ {print $4}'",
        "points": [
          { "temperature_celsius": 30.0, "percent": 25.0 },
          { "temperature_celsius": 70.0, "percent": 70.0 }
        ]
      }
    ],
    "devices": [
      {
        "device_id": "hid:0416:7372:1-5.3",
        "slots": [
          { "slot": 1, "mode": "manual", "percent": 60, "curve": null },
          { "slot": 2, "mode": "manual", "percent": 60, "curve": null },
          { "slot": 3, "mode": "manual", "percent": 60, "curve": null },
          { "slot": 4, "mode": "manual", "percent": 60, "curve": null }
        ]
      }
    ]
  },
  "lcds": [
    {
      "device_id": "serial:ABC123",
      "serial": "ABC123",
      "index": null,
      "media": "color",
      "path": null,
      "fps": null,
      "color": "#ffffff",
      "orientation": 0.0,
      "sensor": null
    }
  ]
}
```

### POST /api/config
Replace the daemon config using the web config model above.

Request:
```json
{
  "default_fps": 30.0,
  "hid_driver": "hidapi",
  "lighting": {
    "enabled": true,
    "openrgb_server": false,
    "openrgb_port": 6743,
    "devices": []
  },
  "fans": {
    "update_interval_ms": 1000,
    "curves": [],
    "devices": []
  },
  "lcds": []
}
```

Notes:
- This endpoint replaces the full daemon config, not a partial section.
- `lighting.brightness_percent` is mapped to the daemon brightness scale `0-4`.
- Fan slots must include exactly slots `1` through `4`.
- `lcds[].device_id` is informational on reads and ignored on writes.
- Relative LCD media/font paths are stored as provided and validated relative to the daemon config directory.

## Profiles

Profiles are stored separately from the daemon config in `profiles.json` next to the daemon
`config.json`.

### GET /api/profiles
List all stored profiles.

Response:
```json
[
  {
    "id": "night-mode",
    "name": "Night Mode",
    "description": "Dim lighting and reduce fan speed",
    "targets": { "mode": "all", "device_ids": [] },
    "lighting": {
      "enabled": true,
      "color": "#223366",
      "effect": "Static",
      "brightness_percent": 15,
      "speed": null,
      "direction": null,
      "scope": null
    },
    "fans": {
      "enabled": true,
      "mode": "manual",
      "percent": 25
    },
    "metadata": {
      "created_at": "2026-03-13T12:00:00Z",
      "updated_at": "2026-03-13T12:00:00Z"
    }
  }
]
```

### POST /api/profiles
Create a profile.

Request:
```json
{
  "id": "night-mode",
  "name": "Night Mode",
  "description": "Dim lighting and reduce fan speed",
  "targets": { "mode": "all", "device_ids": [] },
  "lighting": {
    "enabled": true,
    "color": "#223366",
    "effect": "Static",
    "brightness_percent": 15
  },
  "fans": {
    "enabled": true,
    "mode": "manual",
    "percent": 25
  }
}
```

Notes:
- `id` must be unique and slug-like: lowercase letters, numbers, `-`, `_`.
- At least one of `lighting` or `fans` is required.
- `targets.mode = "all"` requires an empty `device_ids` list.
- `targets.mode = "devices"` requires a non-empty `device_ids` list.

### PUT /api/profiles/:id
Replace an existing profile.

Request body: same as `POST /api/profiles`.

Notes:
- The path id and request-body id must match.
- `metadata.created_at` is preserved; `metadata.updated_at` is refreshed by the backend.

### DELETE /api/profiles/:id
Delete a stored profile.

Response:
```json
{ "deleted": true, "id": "night-mode" }
```

### POST /api/profiles/:id/apply
Apply a stored profile to current devices.

Response:
```json
{
  "profile_id": "night-mode",
  "profile_name": "Night Mode",
  "transaction_mode": "single_config_write",
  "rollback_supported": false,
  "applied_lighting_device_ids": ["wireless:rgb"],
  "applied_fan_device_ids": ["wireless:rgb", "wireless:fan"],
  "skipped_devices": [
    {
      "device_id": "wireless:fan",
      "section": "lighting",
      "reason": "device has no RGB capability"
    }
  ]
}
```

Apply semantics:
- The backend resolves target devices from the live daemon device list.
- The backend loads the current daemon config, merges the profile into it, and writes the result
  back with a single `SetConfig` IPC call.
- Missing explicit target devices fail before any write.
- Capability mismatches are skipped and reported in `skipped_devices`.
- There is no rollback for device-level runtime failures inside the daemon; backend-side writes are
  batched into one config update.

### GET /api/daemon/status
Check if daemon is reachable via IPC.

Response:
```json
{
  "reachable": true,
  "socket_path": "/tmp/lianli-daemon.sock",
  "error": null
}
```

### GET /api/devices
List devices and basic state.

Response:
```json
[
  {
    "id": "hid:0416:7372:1-5.3",
    "name": "TL Fan Controller",
    "family": "TlFan",
    "online": true,
    "capabilities": {
      "has_fan": true,
      "has_rgb": true,
      "has_lcd": false,
      "has_pump": false,
      "fan_count": 4,
      "per_fan_control": false,
      "mb_sync_support": false,
      "rgb_zone_count": null
    },
    "state": {
      "fan_rpms": [1200, 1180, 1210, 1190],
      "coolant_temp": null,
      "streaming_active": false
    }
  }
]
```

### GET /api/devices/:id
Get a single device by id.

Response: same shape as a single device in `/api/devices`.

## Lighting

### GET /api/devices/:id/lighting
Return the stored lighting state (from config) for a device.

Response:
```json
{
  "device_id": "hid:...",
  "zones": [
    {
      "zone": 0,
      "effect": "Static",
      "colors": ["#ff0000"],
      "speed": 2,
      "brightness_percent": 75,
      "direction": "Clockwise",
      "scope": "All"
    }
  ]
}
```

### POST /api/devices/:id/lighting/color
Set a static color (zone default = 0).

Request:
```json
{ "zone": 0, "color": { "hex": "#ff0000" } }
```

### POST /api/devices/:id/lighting/effect
Set an RGB effect.

Request:
```json
{
  "zone": 0,
  "effect": "Rainbow",
  "speed": 3,
  "brightness": 60,
  "color": { "rgb": { "r": 255, "g": 255, "b": 255 } },
  "direction": "Clockwise",
  "scope": "All"
}
```
Notes:
- `speed` is 0-4.
- `brightness` is percent 0-100 and mapped to daemon scale 0-4.

### POST /api/devices/:id/lighting/brightness
Set brightness only (percent 0-100).

Request:
```json
{ "zone": 0, "percent": 80 }
```

## Fans

### GET /api/devices/:id/fans
Return fan config state (from config) + telemetry RPMs if available.

Response:
```json
{
  "device_id": "hid:...",
  "update_interval_ms": 1000,
  "rpms": [1200, 1180, 1210, 1190],
  "slots": [
    { "slot": 1, "mode": "manual", "percent": 60, "pwm": 153, "curve": null },
    { "slot": 2, "mode": "curve", "percent": null, "pwm": null, "curve": "curve-1" }
  ]
}
```

### POST /api/devices/:id/fans/manual
Set fixed fan speed (percent 0-100).

Request:
```json
{ "percent": 60, "slot": 2 }
```

Notes:
- If `slot` is omitted, all 4 slots are set to the same fixed percent.
- Fan control is config-driven; this endpoint updates `FanConfig` via IPC.

## Notes
- Device state is partial: RGB/LCD state is not returned by the daemon; config is used as desired-state source.
- Fan RPMs are taken from telemetry (if available).
- `streaming_active` is a global flag from telemetry.
