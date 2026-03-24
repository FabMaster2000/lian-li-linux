# Device Dashboard

This feature spec describes the device inventory and topology experience introduced in Phase 15.
It covers the Devices overview, controller summaries, quick actions, rename and ordering workflows, and the richer device detail workspace.

- Status: implemented
- Audience: users, operators, testers, reviewers
- Source of truth: current Devices, Device Detail, Lighting, Fans, and Diagnostics routes
- Last reviewed: 2026-03-15
- Related documents:
  - `../user-guides/devices.md`
  - `ui-foundation.md`
  - `../../technical/architecture/device-ordering-and-labeling.md`

## User Goals

The device inventory should now answer four questions quickly:

1. What is connected right now?
2. Which device belongs to which controller or topology group?
3. Which devices need attention or are missing expected telemetry?
4. What is the fastest next action for this specific device?

## Devices Overview

The Devices route is now a working inventory dashboard instead of a plain selection list.

It includes:

- free-text search across display name, raw name, ID, family, controller label, and role
- filter by family
- filter by capability state such as RGB, fan, LCD, pump, or devices that need attention
- grouping by controller or by device family
- controller summary cards
- device cards with quick actions
- a persisted rename and stable ordering workflow

## Controller Summary Cards

Each controller summary card shows:

- controller label
- controller kind
- associated device count
- wireless group count
- distinct channel count
- whether any associated device currently needs attention

These cards are meant to provide the high-level topology view before the user drills into individual devices.

## Device Cards and Quick Actions

Each inventory card shows:

- display name
- controller label
- role and wireless context when relevant
- capability summary
- current mode summary
- health summary

The current quick actions are:

- `Details`
- `Lighting`
- `Fans`
- `Diagnostics`
- `Set static color`
- `Apply profile`
- `Rename / order`

Quick actions are intentionally explicit.
The inventory should support fast actions without hiding where the user is about to land.

## Rename and Ordering Workflow

The `Rename / order` workflow persists these values in backend storage:

- display name
- stable UI order
- physical role
- controller label
- cluster or group label

This means the inventory stays shaped by the user even when the daemon does not provide those concepts natively.

## Device Detail Workspace

The device detail route is now a workspace, not just a raw inspection view.

It shows:

- identity and stable ordering data
- controller and wireless context
- capability coverage
- current lighting summary
- current fan summary
- assigned profiles
- diagnostics summary
- direct links into Lighting, Fans, and Diagnostics for the selected device

## Typical User Flow

1. Open `Devices`.
2. Search or filter until the target group or device is visible.
3. Read the controller summary cards if the topology is unclear.
4. Use quick actions for minor tasks such as color changes or profile application.
5. Open `Details` when deeper capability, diagnostics, or current-state review is needed.
6. Continue into Lighting or Fans with the selected device already carried into the route.

## Expected Behavior for Review and Testing

Reviewers should expect:

- devices to remain in stable order after rename and ordering changes
- controller labels and cluster labels to survive reloads
- device cards to show capability and health context without opening detail
- `Lighting`, `Fans`, and `Diagnostics` links to carry the selected device context forward
- device detail to show profile assignments without requiring a separate trip to the Profiles page

## Current Limitations

- Controller association is still derived from the current device ID model rather than from a full daemon-exposed controller graph.
- `Apply profile` only appears meaningful for profiles that already target the device or all devices.
- The diagnostics route now accepts device focus context, but the richer diagnostics surface belongs to a later phase.
- Wireless topology is still represented pragmatically for the current MVP discovery model and may become more explicit in later phases.
