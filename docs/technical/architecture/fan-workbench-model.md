# Fan Workbench Model

This document defines the Phase 17 data and interaction model for the Fans workbench.
It focuses on target scope, mode selection, curve editing, validation, apply semantics, and warning behavior.

- Status: implemented
- Audience: contributors, maintainers, reviewers
- Source of truth: frontend fan workbench hook and backend fan apply handlers
- Last reviewed: 2026-03-15
- Related documents:
  - `fan-capability-matrix.md`
  - `../backend/fan-curve-api.md`
  - `../../functional/feature-specs/fan-workbench.md`

## Core Concepts

The workbench uses one primary editing device and one apply scope.
That separation is intentional:

- the primary device provides the live fan-state baseline
- the apply scope decides how broadly the current draft is written
- curve library edits are distinct from control-draft edits

## Target Scope Model

The target scope is represented by `target_mode` plus the relevant device identifiers.

| Target mode | Meaning | Backend request fields |
| --- | --- | --- |
| `single` | Apply only to the primary device | `device_id` |
| `selected` | Apply to the explicit selected subset | `device_id`, `device_ids[]` |
| `all` | Apply to every compatible fan-capable device currently in inventory | `device_id` plus backend inventory resolution |

The primary device stays stable even when the target scope expands.
That keeps the live telemetry source and editing baseline predictable.

## Control Draft Model

The current control draft contains:

- `mode`
- `manualPercent`
- `curveName`

`mode` can currently be one of:

- `manual`
- `curve`
- `mb_sync`
- `start_stop`

Only one mode is active in the draft at a time.

## Curve Library Model

A saved curve contains:

- `name`
- `temperature_source`
- `points[]`

Each point contains:

- `temperature_celsius`
- `percent`

The workbench keeps a separate `curveDraft` so users can edit a curve without immediately applying it to the active control draft.

## Validation Rules

The current workbench validates these rules before saving curves:

- curve name is required
- temperature source is required
- at least two points are required in the workbench UI
- temperatures must be valid numbers
- percents must be valid numbers
- percents must remain between `0` and `100`
- temperatures must be strictly ascending
- temperatures must not repeat

The backend also performs its own config validation before persisting curves.

## Apply Semantics

Applying the control draft follows these rules:

1. Resolve targets from `single`, `selected`, or `all`.
2. Validate mode-specific requirements.
3. Write one normalized configuration per eligible target.
4. Return applied devices plus skipped devices with reasons.

Mode-specific behavior:

- `manual`: requires `percent`
- `curve`: requires a saved `curveName`
- `mb_sync`: writes the reserved MB sync marker where supported
- `start_stop`: currently produces skip results because the backend model does not expose start / stop support yet

## Warning State Model

The workbench surfaces warnings or informational notices for these cases:

- no primary device selected
- selected-target mode without compatible selected targets
- high fixed manual speed
- unsaved curve edits while curve mode points at a saved curve
- MB sync on targets that do not report `mb_sync_support`
- current live state already under motherboard sync
- invalid or incomplete curve drafts
- start / stop selected even though the backend currently reports it as unsupported

## Restore and Revert Semantics

The page exposes two safety actions:

- `Reset / Revert`: restore the control draft to the last confirmed backend state
- `Restore defaults`: switch to a safe web-workbench baseline of `manual` at `50%`

`Restore defaults` is intentionally a workbench-safe baseline, not a claim about hardware factory defaults.

## Telemetry Surface

The backend fan state model now provides:

- `active_mode`
- `active_curve`
- `temperature_source`
- `mb_sync_enabled`
- `start_stop_supported`
- `start_stop_enabled`
- per-slot readonly state
- RPM telemetry when available

The workbench treats that telemetry as readonly state and avoids silently discarding a pending draft during background refresh.
