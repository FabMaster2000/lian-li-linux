# Fan Capability Matrix

This document captures the fan-control capabilities currently exposed by the daemon, web backend, and web workbench in Phase 17.
It separates supported workflows from areas that are intentionally visible in the UI but not yet executable end-to-end.

- Status: implemented
- Audience: contributors, maintainers, reviewers
- Source of truth: shared fan types, backend fan handlers, and the Fans workbench frontend
- Last reviewed: 2026-03-15
- Related documents:
  - `fan-workbench-model.md`
  - `../backend/fan-curve-api.md`
  - `../../functional/feature-specs/fan-workbench.md`

## Matrix

| Capability area | Daemon/shared model | Backend API | Web workbench | Current status | Notes |
| --- | --- | --- | --- | --- | --- |
| Manual fixed speed | Supported | Supported | Supported | implemented | One percentage is normalized across the target device fan slots. |
| Fan curve storage | Supported through app config `fan_curves` | Supported with CRUD endpoints | Supported | implemented | Curves are persisted in backend-managed config rather than only in browser state. |
| Temperature source per curve | Supported through `temp_command` | Supported | Supported | implemented | The workbench exposes the raw source string for now. |
| Curve application to one device | Supported | Supported | Supported | implemented | Curve mode applies by named saved curve. |
| Curve application to selected targets | Supported through repeated per-device config writes | Supported | Supported | implemented | The backend reports partial success and skipped targets explicitly. |
| Apply to all compatible fan devices | Supported through backend orchestration | Supported | Supported | implemented | The workbench can reuse the current draft across all fan-capable devices. |
| Motherboard RPM sync | Represented through reserved MB sync curve marker | Supported where device capability allows it | Supported | partial | Unsupported targets are skipped and explained. |
| Start / stop mode | Not exposed by current daemon/backend model | Explicitly reported as unsupported | Visible but not executable | partial | The route keeps the mode visible to document product intent without faking support. |
| Readback of current active mode | Partially derivable from stored config and telemetry | Supported | Supported | implemented | Backend now reports `active_mode`, `active_curve`, and related summary fields. |
| Readback of active curve name | Supported when slot state is uniform | Supported | Supported | implemented | Mixed per-slot curve state is summarized conservatively. |
| Readback of temperature source | Supported through matching stored curve name | Supported | Supported | implemented | Available when the active state resolves to one named curve. |
| Readback of MB sync enabled | Supported | Supported | Supported | implemented | Exposed as `mb_sync_enabled`. |
| Readback of start / stop enabled | Not available | Reported as unsupported | Supported as explicit limitation | partial | The backend returns `start_stop_supported = false` and `start_stop_enabled = false`. |
| Per-slot RPM telemetry | Supported where devices report telemetry | Supported | Supported | implemented | The workbench keeps telemetry readonly and refreshable. |
| Controller- or group-level apply semantics | No dedicated first-class graph yet | Heuristic inventory-driven target resolution | Supported | partial | The backend uses the current inventory model rather than a daemon-native topology graph. |

## Device Family Notes

| Device family or group | Manual mode | Curve mode | MB sync | Start / stop | Notes |
| --- | --- | --- | --- | --- | --- |
| Wired fan controllers such as `Ene6k77` | Supported | Supported | Depends on `mb_sync_support` | Not supported | Current backend handling treats the controller as one fan-control target. |
| `TlFan` controllers and controller-like fan hubs | Supported | Supported | Depends on `mb_sync_support` | Not supported | Controller-specific native constraints are not exposed as a separate web capability graph yet. |
| Pump-capable coolers with fan support | Supported | Supported | Depends on `mb_sync_support` | Not supported | Pump behavior is out of scope for the fan workbench. |
| Wireless fan clusters such as `SlInf`, `Slv3Led`, `Tlv2Led` | Supported | Supported | Device-dependent | Not supported | Current topology remains cluster-level rather than per-fan wireless topology. |
| Devices with no fan capability | Not applicable | Not applicable | Not applicable | Not applicable | These devices are filtered out of the workbench target set. |

## Explicit Limitations

The current phase intentionally leaves these gaps visible:

- no backend or daemon start / stop implementation
- no daemon-native topology graph for controller batches
- no server-side preset catalog beyond named saved curves
- no per-slot authoring workflow for heterogeneous fan-slot configs
- no family-specific capability graph for exact thermal-source constraints

## Review Guidance

Reviewers should expect these behaviors today:

- unsupported MB sync targets are skipped with explicit reasons
- start / stop remains selectable in the UI but results in clear skip reporting
- curve application only uses saved named curves, not unsaved editor drafts
- telemetry refreshes are readonly and should not silently rewrite an unfinished draft
