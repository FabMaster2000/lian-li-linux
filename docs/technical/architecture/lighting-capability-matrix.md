# Lighting Capability Matrix

This document captures the current lighting capabilities that the backend and frontend can expose in Phase 16.
It distinguishes between what the shared RGB model can represent, what the backend currently validates, and where daemon or device-family limitations remain implicit.

- Status: implemented
- Audience: contributors, maintainers, reviewers
- Source of truth: backend lighting handlers, shared RGB types, and the Lighting workbench frontend
- Last reviewed: 2026-03-15
- Related documents:
  - `lighting-workbench-model.md`
  - `../frontend/lighting-page-structure.md`
  - `../../functional/feature-specs/lighting-workbench.md`

## Matrix

| Device family or group | RGB control | Zone model | Multi-device apply | Palette editing | Speed | Direction | Scope | Explicit limitations |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `Ene6k77` wired fan controllers | Supported | Backend uses reported zone count or stored zone config | Supported through workbench orchestration | Supported up to 4 colors in shared RGB effect model | Supported | Partially supported through shared model | Partially supported through shared model | Backend does not expose per-family effect restrictions beyond shared RGB enums |
| `TlFan` controller and `:portN` port devices | Supported | Per-port device entries behave like separate RGB targets | Supported through workbench orchestration | Supported up to 4 colors | Supported | Supported in shared model | Supported in shared model | Controller-specific native constraints are not surfaced explicitly to the web UI yet |
| `Galahad2Trinity` | Supported | Single-device RGB target with pump-specific effects available | Supported | Supported up to 4 colors | Supported | Supported | Supported | Pump-only effects are only considered safe on pump-capable devices |
| `HydroShiftLcd` and `Galahad2Lcd` | Supported | Single-device RGB target plus LCD/pump capabilities | Supported | Supported up to 4 colors | Supported | Supported | Supported | LCD workflows are separate; the lighting workbench does not manage LCD state |
| Wireless RGB clusters such as `SlInf`, `Slv3Led`, `Slv3Lcd`, `Tlv2Led`, `Tlv2Lcd`, `Clv1` | Supported through daemon-discovered wireless cluster entries | Cluster-level zones according to current backend view | Supported | Supported up to 4 colors | Supported | Supported in shared model | Supported in shared model | Current topology remains cluster-level rather than per-fan/per-segment |
| `HydroShift2Lcd`, `Lancool207`, `UniversalScreen`, `DisplaySwitcher`, wireless transport dongles | Not exposed as RGB workbench targets | None | Not applicable | Not applicable | Not applicable | Not applicable | Not applicable | These families are outside the current Lighting workbench target set |

## Supported Operations

The current backend and workbench together support:

- read stored lighting state for one device
- apply one lighting draft to a single device
- apply one lighting draft to a selected device set
- apply one lighting draft to all compatible RGB devices
- apply to one zone or all known zones
- set palette values with up to 4 colors
- set brightness percent
- set speed
- set direction
- set scope
- return partial success information with skipped targets

## Unsupported or Only Partially Modeled Cases

The current phase does not fully model these areas:

- no daemon-exposed family-specific effect capability matrix per exact hardware revision
- no persistent live sync relationship between selected devices after apply; sync is a one-time normalized write
- no explicit backend validation for every family-specific native firmware quirk
- no separate layer-count control in the backend RGB model
- no guarantee that every shared RGB enum maps identically across every hardware family
- no per-LED direct mode in the web workbench even though the shared enum includes `Direct`

## Backend Limitations

The backend currently derives capability warnings from broad device properties rather than a full daemon capability graph.
That means the web layer can validate:

- RGB presence
- pump-only effect restrictions
- zone count where reported
- palette length limits
- brightness and speed ranges

It cannot yet validate:

- exact supported effect list per family and firmware
- exact direction or scope support per family
- whether a device will render a multi-color palette distinctly for every effect

## Daemon and Device-Layer Limitations

The daemon remains authoritative for discovery, device family, RGB presence, and reported zone counts.
It does not currently provide:

- a first-class workbench capability document
- per-effect support metadata
- an explicit topology-aware RGB synchronization primitive
- a dedicated workbench apply endpoint

Phase 16 therefore keeps orchestration in the web backend while documenting these gaps instead of hiding them.
