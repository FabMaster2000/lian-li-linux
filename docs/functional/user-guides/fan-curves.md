# Fan Curves

This guide explains how to create, edit, save, and remove reusable fan curves in the current web workbench.

- Status: implemented
- Audience: users, operators, testers
- Source of truth: current Fans route and backend fan-curve API behavior
- Last reviewed: 2026-03-15
- Related documents:
  - `fans.md`
  - `../feature-specs/fan-workbench.md`
  - `../../technical/backend/fan-curve-api.md`

## When to Use a Curve

Use a saved fan curve when you want fan speed to follow a temperature source instead of staying fixed at one percentage.
A curve is a reusable profile of temperature-to-speed points.

## Create a New Curve

1. Open `Fans`.
2. In `Curve library`, select `New curve`.
3. Enter a curve name.
4. Enter a temperature source string.
5. Add or edit the curve points.
6. Select `Save curve`.

## Edit an Existing Curve

1. Open the `Library selection` dropdown.
2. Choose the saved curve you want to edit.
3. Update the name, temperature source, or points.
4. Review the preview graph.
5. Select `Save curve`.

If you want to keep the original curve unchanged, use `Duplicate` first and save the copy under a new name.

## Delete a Curve

1. Select the saved curve from `Library selection`.
2. Select `Delete curve`.
3. Confirm the dialog.

The backend blocks deletion when the curve is still referenced by stored fan configuration.
If that happens, the page shows the backend error instead of silently removing the curve from the UI.

## Curve Draft Validation

The current workbench requires:

- a non-empty curve name
- a non-empty temperature source
- at least two points in the UI
- valid percentage values between `0` and `100`
- ascending temperature order without duplicates

Validation notices are shown directly in the page before saving.

## Apply a Curve

1. Save the curve first.
2. In `Mode and control editor`, choose `Curve mode`.
3. Select the saved curve in `Saved curve`.
4. Choose the target scope.
5. Select `Apply`.

Unsaved point edits do not automatically change the saved curve that the backend applies.
Save first if the latest point edits should be used.

## Current Limitations

- temperature sources are currently entered as raw text
- there is no separate backend-managed preset catalog beyond saved named curves
- curve application currently uses one saved curve per target rather than per-slot mixed authoring
