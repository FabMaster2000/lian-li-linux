# Lighting

This guide explains how to use the current Lighting Workbench.

- Status: implemented
- Audience: users, operators, testers
- Source of truth: current Lighting route and backend lighting apply behavior
- Last reviewed: 2026-03-15
- Related documents:
  - `devices.md`
  - `profiles.md`
  - `../feature-specs/lighting-workbench.md`

## What the Lighting Workbench Does

The Lighting route is now a structured workbench with one primary editing device and multiple apply scopes.
You can:

- edit a draft from one RGB-capable device
- apply it to only that device
- apply it to a selected device set
- apply it to all compatible RGB devices
- use built-in presets
- save browser-local presets
- review warnings and skipped targets before or after apply

## Typical Workflow

1. Open Lighting.
2. Choose the primary RGB device.
3. Choose the target mode: `single`, `selected`, or `all compatible`.
4. If needed, choose the selected target set.
5. Choose whether the draft applies to the current zone only or all known zones.
6. Adjust effect, palette, brightness, speed, direction, and scope as needed.
7. Review the pending summary and warnings.
8. Select `Apply` or `Apply to all compatible`.
9. Review the last-apply report when any targets were skipped.

## Presets

The page includes curated presets such as:

- Static White
- Static Blue
- Static Red
- Rainbow
- Breathing
- Wave
- Warm Amber
- Performance Red

You can also save the current draft as a browser-local custom preset.
Those saved presets stay in the current browser and are not yet shared through backend storage.

## Pending Changes and Revert

The workbench shows whether the current draft is still pending.
Use:

- `Apply` to write the current draft to the current target scope
- `Reset / Revert` to return to the last confirmed backend state
- `Apply to all compatible` to fan out the current draft without switching the current target-mode tab manually

## Warnings and Unsupported Cases

The page now surfaces unsupported or risky cases explicitly.
Examples include:

- no primary device selected
- no selected targets in selected-target mode
- pump-only effects on non-pump targets
- all-zones apply behavior
- effect and palette combinations that the backend is likely to treat only partially

Skipped targets are listed after apply instead of being silently ignored.

## Current Limitations

- family-specific effect support is still heuristic rather than a full daemon capability graph
- selected-device sync is a one-time shared apply, not a persistent linked group
- custom presets are stored only in the current browser
- the primary device remains the live preview source even when the apply target is broader than one device
