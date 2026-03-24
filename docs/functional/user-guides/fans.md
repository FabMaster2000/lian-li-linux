# Fans

This guide explains how to use the current fan workbench.

- Status: implemented
- Audience: users, operators, testers
- Source of truth: current Fans route and fan backend API behavior
- Last reviewed: 2026-03-15
- Related documents:
  - `fan-curves.md`
  - `devices.md`
  - `profiles.md`
  - `../feature-specs/fan-workbench.md`

## Current Scope

The Fans route now supports:

- one primary fan-capable device for live state and editing baseline
- apply scope selection for `single`, `selected`, and `all compatible`
- manual fixed-speed mode
- saved fan curves and curve mode
- motherboard sync mode where supported
- visible but explicitly unsupported start / stop mode
- readonly telemetry and slot state
- explicit apply-result reporting for skipped targets

## Typical Workflow

1. Open `Fans`.
2. Choose the primary fan-capable device.
3. Choose whether the current draft applies to one device, a selected set, or all compatible targets.
4. Pick the control mode.
5. If using `Curve mode`, save or select the named curve first.
6. Review warnings, telemetry, and the pending summary.
7. Select `Apply`.
8. Review the `Last multi-target apply` result when relevant.

## Current UX Behavior

The page intentionally separates readonly state from editable drafts.
That means:

- live telemetry can refresh while you continue editing
- saved curves are distinct from unsaved curve-draft edits
- unsupported targets are reported explicitly instead of being silently ignored
- restore and revert actions stay available before apply

## Telemetry Surface

The current route shows these readonly values for the primary device when available:

- active mode
- active curve
- temperature source
- update interval
- RPM telemetry
- motherboard sync enabled state
- start / stop support state
- per-slot readonly values

## Safe Operation Notes

The workbench currently warns for cases such as:

- no primary device selected
- no targets selected in selected-target mode
- high fixed manual speeds
- invalid curve drafts
- motherboard sync limits on unsupported targets
- start / stop selected before backend support exists

## Current Limitations

- start / stop remains a documented placeholder rather than an executable backend mode
- the route does not expose per-slot authoring for heterogeneous fan-slot configs
- temperature source selection is still a text field rather than a curated picker
