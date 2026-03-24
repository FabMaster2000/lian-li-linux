# Profiles

This guide explains how to use the current profile library.

- Status: partial
- Audience: users, operators, testers
- Source of truth: current Profiles route and backend profile API
- Last reviewed: 2026-03-14
- Related documents:
  - `lighting.md`
  - `fans.md`
  - `../../technical/architecture/domain-model.md`

## Current Scope

The current profile system lets you:

- create a stored profile
- target all devices or selected devices
- include lighting settings
- include manual fan settings
- apply a profile to the current live inventory
- delete a stored profile

Profiles are stored in the backend and are separate from daemon-native config ownership.

## Typical Workflow

1. Open Profiles.
2. Enter a profile ID and name.
3. Choose whether the profile applies to all devices or selected devices.
4. Optionally add a description.
5. Enable lighting and or fan content.
6. Fill in the desired lighting or manual fan values.
7. Create the profile.
8. Apply the profile from the library when needed.

## Current UX Behavior

- profiles use a backend-backed library rather than frontend-only local state
- the page shows apply results and skipped targets after profile application
- selected-device mode requires explicit device checkboxes
- delete and apply actions surface success or failure messages

## Current Limitations

- no import or export
- no profile tags or categories
- no built-in system presets
- no LCD/media profile content
- no compatibility matrix beyond the current apply-time validation and skip reporting
