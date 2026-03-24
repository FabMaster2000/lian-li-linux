# Fan Curve API

This document describes the current backend API surface for fan state, fan curve CRUD, and batch fan workbench apply in Phase 17.

- Status: implemented
- Audience: contributors, maintainers, API reviewers
- Source of truth: backend `routes/mod.rs`, `handlers/mod.rs`, and shared fan config model
- Last reviewed: 2026-03-15
- Related documents:
  - `../architecture/fan-workbench-model.md`
  - `../architecture/fan-capability-matrix.md`
  - `../../functional/feature-specs/fan-workbench.md`

## Endpoints

### `GET /api/devices/:id/fans`

Returns the current readonly fan state for one device.

Response fields include:

- `device_id`
- `update_interval_ms`
- `rpms`
- `slots[]`
- `active_mode`
- `active_curve`
- `temperature_source`
- `mb_sync_enabled`
- `start_stop_supported`
- `start_stop_enabled`

### `POST /api/devices/:id/fans/manual`

Applies one fixed manual percentage to the selected device.

Request body:

```json
{
  "percent": 55,
  "slot": null
}
```

Notes:

- `percent` must stay between `0` and `100`
- `slot` is optional and must be between `1` and `4` when provided
- the current web workbench uses whole-device writes rather than per-slot authoring

### `GET /api/fan-curves`

Lists saved fan curves from backend-managed config.

### `POST /api/fan-curves`

Creates one saved fan curve.

Request body:

```json
{
  "name": "Balanced curve",
  "temperature_source": "sensors temperature --source coolant",
  "points": [
    { "temperature_celsius": 28, "percent": 25 },
    { "temperature_celsius": 40, "percent": 45 },
    { "temperature_celsius": 58, "percent": 72 }
  ]
}
```

### `PUT /api/fan-curves/:name`

Updates an existing saved curve.
This endpoint also supports rename semantics by changing `body.name`.

### `DELETE /api/fan-curves/:name`

Deletes an existing saved curve.
Deletion is rejected when the named curve is still referenced by one or more stored fan slots.

### `POST /api/fans/apply`

Applies the current workbench draft across `single`, `selected`, or `all` fan targets.

Request body:

```json
{
  "target_mode": "selected",
  "device_id": "wireless:fan-primary",
  "device_ids": ["wireless:fan-primary", "wireless:fan-secondary"],
  "mode": "curve",
  "curve": "Balanced curve"
}
```

Manual example:

```json
{
  "target_mode": "single",
  "device_id": "wireless:fan-primary",
  "mode": "manual",
  "percent": 60
}
```

## Response Model for Batch Apply

`POST /api/fans/apply` returns:

- `target_mode`
- `mode`
- `requested_device_ids[]`
- `applied_devices[]`
- `skipped_devices[]`

`applied_devices[]` reuses the enriched fan-state model.
`skipped_devices[]` always contains explicit `device_id` plus `reason`.

## Validation and Error Behavior

The backend currently validates:

- target mode names
- fan mode names
- required `percent` for manual mode
- required `curve` for curve mode
- percent range `0..100`
- existence of referenced saved curves
- presence of selected target IDs in selected-target mode
- slot-range constraints for the single-device manual endpoint
- curve deletion safety when a curve is still referenced

## Unsupported Mode Behavior

`start_stop` is intentionally visible in the workbench but not supported by the current backend model.
The batch apply endpoint therefore reports each targeted device as skipped with an explicit unsupported reason instead of silently accepting or faking the request.

## Event Semantics

The backend currently emits fan-related events after successful writes:

- `fan.changed` after manual writes or workbench apply
- `config.changed` after fan-curve create, update, or delete

The web workbench uses those events to refresh readonly state and curve library data.

## Implementation Notes

- saved curves live in full app config and therefore use config-save validation
- fan workbench apply writes through the daemon fan-config channel
- MB sync is represented through the reserved shared fan marker, not a separate backend config tree
