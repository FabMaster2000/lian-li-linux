# Target Information Architecture

This document defines the target top-level information architecture introduced in Phase 14.
It turns the MVP route set into a coherent control-suite navigation model while keeping later feature phases explicitly visible.

- Status: implemented
- Audience: contributors, maintainers
- Source of truth: Phase 14 route structure and navigation model
- Last reviewed: 2026-03-21
- Related documents:
  - `current-information-architecture.md`
  - `design-system.md`
  - `../architecture/frontend-architecture.md`

## Design Principles

- Top-level navigation should describe product areas, not implementation artifacts.
- Current and future workbenches should share the same shell and page anatomy.
- Placeholder pages are acceptable when they reserve a real navigation target and make feature status explicit.
- Device-specific drill-down remains nested under the broader Devices workflow.
- Documentation and support should be discoverable from inside the application shell.

## Target Top-Level Navigation

| Area | Route | Current state |
| --- | --- | --- |
| Dashboard | `/` | implemented |
| Devices | `/devices` | implemented |
| Lighting | `/rgb` | implemented |
| Fans | `/fans` | implemented |
| Wireless Sync | `/wireless-sync` | implemented |
| LCD / Media | `/lcd-media` | placeholder redirect |
| Profiles | `/profiles` | implemented |
| Diagnostics | `/diagnostics` | implemented (scaffold) |
| Settings | `/settings` | implemented |
| Help / Docs | `/help` | implemented |

## Nested and Supporting Routes

- `/devices/:deviceId` remains the detail route for the selected device.
- Device detail is intentionally discovered through Devices rather than treated as a separate top-level area.
- Future capability-specific or multi-step routes should remain under their owning top-level product area.

## Standard Page Anatomy

Every page should now follow this sequence:

1. page header
2. context summary strip
3. primary work area
4. secondary information area
5. explicit action bar when a draft or workflow action needs confirmation

This pattern is now present across the live routes and the new placeholder foundations.

## Why Placeholder Routes Exist Now

LCD / Media is the only remaining placeholder redirect.
All other product areas now have active routes with live pages, even if some (like Diagnostics) are still scaffold-level.

## Expected Benefits

- clearer orientation for operators
- room for later workbenches without another full shell rewrite
- better separation between current inventory/detail flows and future controller-specific surfaces
- explicit feature status visibility instead of hidden or missing destinations
