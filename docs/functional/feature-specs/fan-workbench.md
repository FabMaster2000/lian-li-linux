# Fan Workbench

This feature spec describes the Phase 17 Fans workbench.
It covers batch target selection, saved fan curves, supported modes, telemetry readback, and the current safety model for unsupported cases.

- Status: implemented
- Audience: users, operators, testers, reviewers
- Source of truth: current Fans route and backend fan apply behavior
- Last reviewed: 2026-03-15
- Related documents:
  - `../user-guides/fans.md`
  - `../user-guides/fan-curves.md`
  - `../../technical/architecture/fan-workbench-model.md`

## User Goals

The Fans route should now answer these questions quickly:

1. Which device is my current cooling baseline?
2. Will the pending draft apply to one device, a selected set, or all compatible devices?
3. Which saved curve am I editing or applying?
4. Which targets will be skipped because of capability or support limits?

## Core Workbench Behavior

The current route is now a structured workbench instead of a single manual slider.
It includes:

- a primary editing device
- target modes for `single`, `selected`, and `all compatible`
- manual fixed-speed mode
- curve mode based on saved named curves
- motherboard sync mode where supported
- visible but explicitly unsupported start / stop mode
- curve create, rename, duplicate, delete, and point editing
- pending-change visibility
- readonly telemetry and slot state
- explicit apply-result reporting

## Supported Modes

### Manual

Manual mode writes one fixed percentage across the targeted device fan slots.
The workbench warns when the chosen fixed speed is unusually high.

### Curve

Curve mode applies a saved named curve.
Unsaved editor changes stay local until the user saves the curve first.

### Motherboard Sync

Motherboard sync is exposed as a first-class mode.
Targets that do not report support are skipped with an explicit reason.

### Start / Stop

Start / stop is visible in the mode list so the route structure matches the intended product surface.
The current backend does not execute it yet, so the apply result reports the limitation clearly.

## Curve Library Expectations

Users should be able to:

- create a new curve draft
- rename a curve
- duplicate a curve as a new draft
- edit curve points
- preview the curve shape
- save a curve
- delete a saved curve after confirmation

The backend may still reject deletion if the curve is in use by stored fan-slot config.

## Telemetry and Readback

The page now shows readonly state for the primary device, including:

- active mode
- active curve
- temperature source
- update interval
- RPM telemetry
- whether motherboard sync is currently active
- whether start / stop is supported or enabled

## Safety Model

The current route intentionally favors explicit warnings over hidden heuristics.
Users should expect notices for cases such as:

- no primary device selected
- no selected targets in selected-target mode
- high fixed manual speed
- invalid curve drafts
- motherboard sync limits on unsupported targets
- current live state already under motherboard sync
- start / stop chosen before backend support exists

## Review and Test Expectations

Reviewers should expect:

- the generic Fans route to require explicit device selection
- saved curves to remain separate from unsaved editor changes
- `Apply` to reuse the current target mode
- `Apply to all compatible` to expand scope without leaving the page
- skipped targets to appear in the last-apply report with reasons
- readonly telemetry refreshes to stay visible while edits remain local until apply

## Current Limitations

- start / stop is still documented as unsupported by the backend model
- the workbench applies one normalized config per target rather than exposing heterogeneous per-slot authoring
- topology is inventory-driven rather than backed by a daemon-native controller graph
- the curve editor currently exposes raw temperature-source text rather than a curated source picker
