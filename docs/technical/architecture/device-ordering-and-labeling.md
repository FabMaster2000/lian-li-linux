# Device Ordering and Labeling

This document explains how the backend derives the web-facing device topology and where user-controlled presentation metadata is stored.
It covers display names, stable ordering, controller labels, cluster labels, and the boundary between daemon-owned and backend-owned state.

- Status: implemented
- Audience: contributors, maintainers
- Source of truth: backend device inventory handlers and profile-store-backed presentation metadata
- Last reviewed: 2026-03-15
- Related documents:
  - `domain-model.md`
  - `source-of-truth.md`
  - `../../functional/feature-specs/device-dashboard.md`

## Problem Statement

The daemon exposes the discovered device inventory, telemetry, and low-level capabilities.
It does not currently expose a stable user-facing presentation layer for labels, UI ordering, controller naming, or cluster naming.

Phase 15 therefore introduces a backend-owned presentation layer that sits on top of daemon discovery without replacing daemon truth for hardware identity and capabilities.

## Ownership Split

The system now uses this split:

- The daemon remains authoritative for discovered devices, raw device IDs, family, capability flags, wireless channel, and live telemetry.
- The backend is authoritative for presentation metadata that the user can edit from the web UI.
- The frontend renders the combined view model returned by the backend and should not maintain a separate source of truth for labels or ordering.

## Web-Facing Inventory Model

The web-facing `DeviceView` now includes these additional fields:

- `display_name`: effective user-visible name
- `ui_order`: stable numeric ordering key used by inventory sorting
- `physical_role`: user-facing role such as `Wireless cluster` or `Port 2`
- `capability_summary`: precomputed short summary for overview surfaces
- `current_mode_summary`: precomputed current-state summary for overview surfaces
- `controller`: derived controller association with `id`, `label`, and `kind`
- `wireless`: optional wireless transport, channel, and group metadata
- `health`: normalized health level plus summary text

The backend sorts devices by:

1. `ui_order`
2. controller label
3. display name
4. raw device ID

## Derivation Rules

### Controller Association

The backend derives controller identity from the daemon device ID:

- Wireless devices map to a shared controller identity: `wireless:mesh`
- Wired port devices map to the base device ID before the `:portN` suffix
- Standalone devices use their own device ID as controller identity

This is intentionally pragmatic for the current MVP topology.
It avoids inventing controller records that the daemon does not yet expose explicitly.

### Wireless Metadata

Wireless metadata is split as follows:

- `wireless.channel` comes from daemon discovery when available
- `wireless.group_id` defaults to the device ID for the current cluster-level device model
- `wireless.group_label` is backend-owned presentation metadata and defaults to the device display name

### Physical Role

When no user override exists, the backend assigns a default role using ID and capability heuristics:

- `Port N` for wired port-scoped devices
- `Wireless cluster` for current wireless device entries
- device-type-specific fallbacks such as `Pump device` or `LCD cooling device`

## Persistent Metadata

The backend persists presentation metadata in the backend profile store file.
That file now contains more than profiles:

- `profiles`
- `device_presentations`
- `controller_labels`

This keeps backend-owned user intent in one JSON-backed store and avoids depending on daemon config support that does not exist yet.

### Device Presentation Record

Each record is keyed by `device_id` and may store:

- `display_name`
- `ui_order`
- `physical_role`
- `cluster_label`

### Controller Label Record

Each record is keyed by `controller_id` and stores:

- `label`

## API Surface

Phase 15 adds one presentation mutation endpoint:

## PUT /api/devices/:id/presentation

Request body:

```json
{
  "display_name": "Desk Cluster",
  "ui_order": 25,
  "physical_role": "Rear intake cluster",
  "controller_label": "Desk wireless",
  "cluster_label": "Desk Cluster"
}
```

Behavior:

- validates that the device exists in the current daemon inventory
- writes presentation metadata into backend storage
- emits `device.updated` so inventory-driven pages can refresh
- returns the updated `DeviceView`

The existing read endpoints now return the enriched inventory model:

- `GET /api/devices`
- `GET /api/devices/:id`

## Why This Design

This design was chosen because it preserves the right ownership boundary:

- hardware discovery stays in the daemon
- user-managed presentation stays in the backend
- the frontend receives one normalized document instead of re-deriving controller and label rules locally

Alternatives that were rejected:

- storing labels only in frontend local state, because they would not survive reloads or be visible to other sessions
- writing presentation metadata into daemon config, because current daemon support does not model these UI concerns
- introducing a separate database just for labels and ordering, because the existing JSON-backed backend storage is sufficient for the current scope

## Current Limitations

- Wireless controller identity is intentionally collapsed into a shared `wireless:mesh` model for the current implementation.
- Group and cluster naming are scoped to the current cluster-level device model and may change if later phases expose more explicit topology primitives.
- Health summaries are intentionally high-level and not yet a replacement for the later diagnostics phase.
- The profile store path name still reflects its historical purpose even though it now stores more than profiles.
