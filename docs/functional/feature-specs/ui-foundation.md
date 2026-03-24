# UI Foundation

This feature spec describes the new frontend shell and interaction structure introduced in Phase 14.
It explains the user-visible behavior of the redesign without going into implementation details.

- Status: implemented
- Audience: users, admins, testers, reviewers
- Source of truth: current frontend shell and route behavior
- Last reviewed: 2026-03-14
- Related documents:
  - `../user-guides/dashboard.md`
  - `../user-guides/devices.md`
  - `../../technical/frontend/target-information-architecture.md`

## User-Facing Goals

The redesign should make the web controller feel like one coherent control suite instead of a collection of disconnected MVP pages.

Users should now experience:

- a stable top-level navigation model
- clearer route ownership
- more consistent page structure
- better visibility of current system state
- explicit separation between live routes and planned future workbenches

## Visible Navigation Model

The shell now exposes these top-level areas:

- Dashboard
- Devices
- Lighting
- Fans
- Wireless Sync
- LCD / Media
- Profiles
- Diagnostics
- Settings
- Help / Docs

Some of these routes are already operational.
Others are intentionally placeholder foundations so users can see the product shape early without mistaking future phases for missing navigation.

## Shared Page Structure

Pages now follow a more consistent visual sequence:

1. a page header with actions
2. a summary strip with high-signal context
3. a primary work area
4. supporting context or secondary information
5. an explicit action bar where apply or save workflows matter

## Status Visibility

The shell uses badges and notices to make status explicit:

- navigation badges show whether a route is ready, next, or planned
- global notices surface API, daemon, or websocket issues
- page notices surface errors, warnings, success states, and staged placeholder messaging

## Responsive Behavior

The shell is still desktop-first, but it now remains structurally stable on smaller screens:

- the sidebar collapses into a stacked layout
- multi-column summary strips and work areas compress without changing route order
- action controls remain visible and usable on tablet and smaller screens

## Current Limitations

- Wireless Sync and LCD / Media are shell foundations, not feature-complete workbenches yet
- Diagnostics and Settings are now dedicated navigation targets, but their richer operational workflows belong to later phases
- Help / Docs is an in-app entry point, not a full documentation renderer
