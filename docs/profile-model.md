# Profile Model

## Scope
This document defines the web-level profile model for Phase 6.3.
Profiles are intentionally independent from the daemon's technical `AppConfig`.
They represent user-facing presets such as `Silent`, `Performance`, `White Static`,
or `Night Mode`.

## Current Codebase Reality
- No profile storage exists today.
- No profile-related IPC or GUI flow exists.
- The daemon already exposes the primitives needed to apply profile content:
  - lighting writes through `/api/devices/:id/lighting/*`
  - fan writes through `/api/devices/:id/fans/manual`
  - config reads/writes through `/api/config`

Conclusion:
- profiles should be modeled as user intent, not as raw daemon config snapshots
- applying a profile should orchestrate existing API operations
- profile persistence should live outside daemon config

## Design Goals
- user-friendly and stable
- independent from low-level daemon schema
- reusable across multiple devices
- partial by design: a profile can change lighting, fans, or both
- explicit targets: a profile can apply to all devices or a selected subset
- forward-compatible with future fan curves, LCD presets, and device-specific overrides

## Proposed Web Model

```json
{
  "id": "night-mode",
  "name": "Night Mode",
  "description": "Low brightness lighting and quiet fans",
  "targets": {
    "mode": "all",
    "device_ids": []
  },
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
    "created_at": "2026-03-13T00:00:00Z",
    "updated_at": "2026-03-13T00:00:00Z"
  }
}
```

## Fields

### Profile
```text
Profile {
  id: string,
  name: string,
  description: string,
  targets: ProfileTargets,
  lighting?: LightingProfile,
  fans?: FanProfile,
  metadata: ProfileMetadata
}
```

Field notes:
- `id`
  - stable profile identifier for API and storage
  - slug-like format recommended, e.g. `silent`, `white-static`, `night-mode`
- `name`
  - user-facing display name
- `description`
  - optional human-readable explanation
- `targets`
  - defines where the profile applies
- `lighting`
  - optional: omitted means "do not change lighting"
- `fans`
  - optional: omitted means "do not change fan settings"
- `metadata`
  - storage metadata, not part of daemon mapping

### ProfileTargets
```text
ProfileTargets {
  mode: "all" | "devices",
  device_ids: [string]
}
```

Rules:
- `mode = "all"`
  - apply to all compatible devices
  - `device_ids` should be empty
- `mode = "devices"`
  - apply only to listed devices
  - `device_ids` must be non-empty

Rationale:
- keep MVP targeting simple and explicit
- avoid family-based or rule-based selectors until real UI demand exists

### LightingProfile
```text
LightingProfile {
  enabled: bool,
  color?: string,              // #RRGGBB
  effect?: string,             // e.g. Static, Rainbow
  brightness_percent?: u8,     // 0..100
  speed?: u8,                  // 0..4
  direction?: string,
  scope?: string
}
```

Rules:
- `enabled = false`
  - treat as "turn lighting off" if supported by the target flow
- `color`
  - only needed for static or color-based effects
- `effect`
  - use the same naming conventions as the web API
- `brightness_percent`
  - mapped to daemon brightness scale `0..4`
- `speed`, `direction`, `scope`
  - optional effect modifiers

Rationale:
- profiles should store one consistent lighting intent
- device-specific zone structures stay in API/config layer, not in profile storage

### FanProfile
```text
FanProfile {
  enabled: bool,
  mode: "manual",
  percent: u8
}
```

Rules:
- MVP supports manual percent only
- `percent` is required when `mode = "manual"`
- future extension can add:
  - `mode = "curve"`
  - `curve_id`
  - `mode = "config_snapshot"`

Rationale:
- keep MVP profile fan behavior aligned with the current backend fan API
- avoid baking daemon fan curve internals into the initial profile format

### ProfileMetadata
```text
ProfileMetadata {
  created_at: string,  // ISO 8601
  updated_at: string   // ISO 8601
}
```

Rationale:
- useful for UI sorting, auditing, and future sync/export

## Apply Semantics
Profiles should be applied as an orchestration layer over the existing backend API.

Recommended semantics:
- resolve targets first
- if `lighting` exists:
  - apply lighting settings to each compatible target device
- if `fans` exists:
  - apply fan settings to each compatible target device
- omitted sections do not overwrite current state

Example:
- a `White Static` profile may include only `lighting`
- a `Silent` profile may include only `fans`
- a `Night Mode` profile may include both

## MVP Backend Implementation
- profiles are stored in `profiles.json` next to the daemon `config.json`
- storage decision details are documented in `docs/profile-storage.md`
- the backend resolves targets from the live daemon device list at apply time
- the backend merges profile content into the current daemon config and writes it back with one
  `SetConfig` call
- explicit missing target devices fail before writing
- incompatible capabilities are skipped and reported per device/section
- rollback is not supported for daemon-internal runtime failures after config acceptance

## Mapping to Existing API

### Lighting
Map profile lighting to:
- `POST /api/devices/:id/lighting/color`
- `POST /api/devices/:id/lighting/effect`
- `POST /api/devices/:id/lighting/brightness`

Recommended order:
1. set effect if present
2. set color if present
3. set brightness if present

### Fans
Map profile fan settings to:
- `POST /api/devices/:id/fans/manual`

## Validation Rules
- `id` must be unique within profile storage
- `name` must not be empty
- `targets.mode = "devices"` requires at least one `device_id`
- `lighting.color` must be valid `#RRGGBB` if present
- `lighting.brightness_percent` must be `0..100`
- `lighting.speed` must be `0..4`
- `fans.percent` must be `0..100`
- at least one of `lighting` or `fans` should be present

## Example Profiles

### Silent
```json
{
  "id": "silent",
  "name": "Silent",
  "description": "Lower fixed fan speed, keep lighting unchanged",
  "targets": { "mode": "all", "device_ids": [] },
  "fans": {
    "enabled": true,
    "mode": "manual",
    "percent": 30
  }
}
```

### Performance
```json
{
  "id": "performance",
  "name": "Performance",
  "description": "Higher fixed fan speed for cooling",
  "targets": { "mode": "all", "device_ids": [] },
  "fans": {
    "enabled": true,
    "mode": "manual",
    "percent": 80
  }
}
```

### White Static
```json
{
  "id": "white-static",
  "name": "White Static",
  "description": "Static white lighting for all RGB-capable devices",
  "targets": { "mode": "all", "device_ids": [] },
  "lighting": {
    "enabled": true,
    "color": "#ffffff",
    "effect": "Static",
    "brightness_percent": 80
  }
}
```

### Night Mode
```json
{
  "id": "night-mode",
  "name": "Night Mode",
  "description": "Dim lighting and reduced fan noise",
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

## Non-Goals for MVP
- raw daemon config snapshots as profile payloads
- device-family rule engine
- LCD/GIF/video profile content
- transactional rollback logic
- multi-step dependency resolution between profile sections

These can be added later if real usage requires them.

## Summary
- profiles are a user-facing preset layer above config and device APIs
- they should stay simpler than daemon config
- they should be stored separately from daemon `config.json`
- they should support partial application and explicit targets
- MVP should focus on lighting plus manual fan settings
