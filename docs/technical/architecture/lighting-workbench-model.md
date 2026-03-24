# Lighting Workbench Model

This document defines the current Lighting Workbench model used in Phase 16.
It describes the target scope, draft structure, apply behavior, preview model, and partial-success semantics without overstating daemon support.

- Status: implemented
- Audience: contributors, maintainers
- Source of truth: frontend lighting workbench hook and backend lighting apply handler
- Last reviewed: 2026-03-15
- Related documents:
  - `lighting-capability-matrix.md`
  - `../frontend/lighting-page-structure.md`
  - `../../functional/feature-specs/lighting-workbench.md`

## Target Scope

The workbench separates one primary editing device from the apply target scope.
That distinction is intentional.

- The primary device is the one device whose live zone state is loaded for preview and draft seeding.
- The target mode decides where the current draft is written.
- Target modes are `single`, `selected`, and `all`.

This keeps one stable editing context even when the user applies to many devices.

## Device Selection

The model uses two device-selection layers:

- `selectedDeviceId`: the primary editing device
- `selectedDeviceIds`: the optional multi-select target set for `selected` mode

In `all` mode, the backend targets all currently compatible RGB devices.
In `selected` mode, the backend applies only to the explicit selected set.
In `single` mode, only the primary device is targeted.

## Zone and Group Selection

The draft also separates the editing zone from the apply zone mode:

- `zone`: the currently focused zone index
- `zoneMode = active`: apply only to the selected zone
- `zoneMode = all_zones`: apply the current draft to every known zone on each targeted device

There is no richer group or segment assignment model yet.
That will depend on later daemon capability work.

## Color Model

The workbench color model is palette-based.
It uses the shared RGB effect structure already present in the backend.

- palette size: 1 to 4 colors
- palette storage: ordered list of hex colors
- palette editing: available only for effects that the current UI heuristic marks as palette-aware
- single-color effects keep only the first palette entry in the editing surface

The backend accepts palette arrays for the new workbench apply path and for the single-device effect endpoint.

## Effect Model

The workbench effect model is a shared RGB-mode catalog with UI heuristics layered on top.
The draft contains:

- effect name
- optional palette
- optional speed
- optional direction
- optional scope

The backend validates the shared enum and basic ranges.
It does not yet expose an exact family-specific effect capability graph.
The frontend therefore shows capability warnings rather than pretending exact per-family precision.

## Brightness Model

Brightness is represented as a 0-100 percent draft field in the workbench.
The backend converts it into the shared 0-4 brightness level used by the RGB config.

This means:

- the UI can remain user-friendly
- the stored config still matches the existing daemon/shared model

## Apply Strategy

Phase 16 adds a dedicated backend apply path for workbench lighting writes.
The strategy is:

1. resolve target devices from target mode
2. validate obvious unsupported cases
3. write one normalized RGB config update covering all applicable targets
4. emit per-device lighting change events
5. return applied devices plus skipped targets

`sync selected devices` is currently a one-time synchronized write across the current selection.
It is not a persistent future linkage.

## Preview State

The workbench preview model is intentionally split:

- live preview: readonly backend state for the primary device and active zone
- pending preview: the current unsaved draft summary and palette swatch

Readonly background refreshes update the live preview and baseline state without overwriting draft edits.

## Pending Changes State

The workbench keeps a baseline draft derived from the last confirmed backend state.
The current draft is compared against that baseline.

This enables:

- unsaved-changes indication
- explicit Apply
- Reset / Revert to the last confirmed state
- stable pending summary text in the action bar

## Save-as-Preset Model

Built-in presets are first-party frontend definitions.
Custom saved presets are currently browser-local and stored in local storage.

This is deliberate for the current phase because there is no backend preset library model yet.
The limitation is documented instead of hidden.

## Partial Success Handling

The backend apply response returns:

- requested device IDs
- applied device states
- skipped devices with reasons
- target mode and zone mode
- whether sync was requested

The frontend uses this to report:

- fully successful apply
- partial success with skipped targets
- zero-success cases where nothing compatible could be updated

## Known Model Boundaries

- no explicit per-family effect graph from the daemon
- no persistent sync group concept after apply
- no separate layer-count field in the backend model
- no server-side custom preset storage yet
- no per-device preview fan-out for every targeted device; the primary device remains the live preview source
