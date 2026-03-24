# Lighting Workbench

This feature spec describes the Phase 16 Lighting Workbench.
It covers target modes, presets, palette editing, pending changes, multi-device apply, and the current handling of unsupported targets.

- Status: implemented
- Audience: users, operators, testers, reviewers
- Source of truth: current Lighting route and backend lighting apply behavior
- Last reviewed: 2026-03-15
- Related documents:
  - `../user-guides/lighting.md`
  - `ui-foundation.md`
  - `../../technical/architecture/lighting-workbench-model.md`

## User Goals

The lighting surface should now answer these questions quickly:

1. Which device am I editing right now?
2. Will this draft apply to one device, a selected set, or all compatible devices?
3. What will change if I press Apply?
4. Which targets will be skipped or only partially supported?

## Core Workbench Behavior

The Lighting route is now a structured workbench instead of a simple per-device form.
It includes:

- a primary editing device
- target modes for `single`, `selected`, and `all compatible`
- zone-only or all-zones apply
- built-in presets
- browser-local custom presets
- palette editing
- speed, direction, scope, and brightness controls where exposed
- an explicit pending changes bar
- partial-success reporting

## Target Modes

The page separates the editing baseline from the apply scope.
That means the user can:

- build a draft from one device's current live state
- keep that device as the live preview source
- still apply the draft to a selected group or to all compatible RGB devices

## Presets

Built-in presets now include:

- Static White
- Static Blue
- Static Red
- Rainbow
- Breathing
- Wave
- Warm Amber
- Performance Red

Users can also save custom presets locally in the browser.
Those presets are not yet shared across browsers or users.

## Multi-Device Apply

The workbench now supports:

- single-device apply
- selected-device apply
- apply to all compatible RGB devices
- one-time sync apply across the selected device set
- partial success reporting when some targets are skipped

Skipped targets are reported explicitly with reasons instead of being silently ignored.

## Pending Changes and Revert

The page tracks whether the current draft differs from the last confirmed backend state.
This enables:

- visible unsaved-changes indication
- explicit Apply
- Reset / Revert
- a preview summary before write

## Capability Warnings

The current frontend and backend show warnings for cases such as:

- no primary device selected
- no devices chosen in selected-target mode
- pump-only effects on non-pump targets
- all-zones apply scope
- palette values on effects that are likely to ignore them

These warnings are intentionally explicit because the daemon does not yet expose a fully detailed per-family effect graph.

## Expected Behavior for Review and Testing

Reviewers should expect:

- the generic Lighting route to require a primary RGB device before applying
- preset buttons to update the pending draft
- dirty-state indication to appear after edits
- `Apply to all compatible` to reuse the current draft without forcing a target-mode switch first
- skipped targets to appear in a last-apply report when the backend reports them
- live backend refreshes to update readonly state without wiping draft edits

## Current Limitations

- exact per-family effect support remains a documented heuristic, not a daemon capability graph
- sync selected devices is a one-time shared apply, not a persistent future relationship
- custom presets are browser-local only
- layer count is not exposed because the backend RGB model does not currently represent it separately
