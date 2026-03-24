# Current Information Architecture

This document audits the frontend information architecture before and during the Phase 14 redesign.
It describes the current route map, page responsibilities, loading patterns, and the UX inconsistencies that motivated the new application shell and design-system foundation.

- Status: implemented
- Audience: contributors, maintainers
- Source of truth: current frontend route and page implementation
- Last reviewed: 2026-03-21
- Related documents:
  - `target-information-architecture.md`
  - `design-system.md`
  - `../architecture/frontend-architecture.md`

## Current Route Map

| Route | Page | Current state |
| --- | --- | --- |
| `/` | Dashboard | implemented |
| `/devices` | Devices | implemented |
| `/devices/:deviceId` | Device Detail | implemented |
| `/rgb` | Lighting | implemented |
| `/fans` | Fans | implemented |
| `/wireless-sync` | Wireless Sync | implemented |
| `/profiles` | Profiles | implemented |
| `/diagnostics` | Diagnostics | implemented (scaffold with live data) |
| `/settings` | Settings | implemented |
| `/help` | Help / Docs | implemented |
| `/lcd-media` | LCD / Media | redirects to `/` (placeholder) |
| `/lighting` | — | legacy redirect to `/rgb` |

## Current Page Responsibilities

- Dashboard: system summary, device inventory, runtime paths, and route entry points
- Devices: explicit device inventory with search and family filtering
- Device Detail: current device identity, capabilities, lighting snapshot, and fan snapshot
- Lighting: cluster-wide RGB editing with effect/color/brightness/speed controls and drag-route reordering
- Fans: cluster-wide fan control with manual percentage, curve mode, and batch apply
- Wireless Sync: pair and manage wireless clusters in a two-tab workflow
- Profiles: backend-stored preset creation, application, and deletion
- Diagnostics: runtime health display with daemon, API, and event-stream status
- Settings: system configuration and links to active hardware tools
- Help / Docs: tabbed documentation navigator for technical and functional docs

## Current Loading Patterns

- Dashboard loads devices, daemon status, and runtime data together through a dedicated hook.
- Devices fetches the current inventory on mount and on manual refresh.
- Device Detail loads device, lighting, and fan state for one explicit ID.
- Lighting and Fans keep readonly backend state refreshable without discarding the local draft.
- Profiles loads the profile library and current device inventory, then manages a separate local draft for creation.
- Global notices use system-status polling plus websocket state to surface backend or daemon problems.

## Duplicated or Inconsistent UX Elements Before the Redesign

- The sidebar navigation used MVP labels such as `Device Detail` even though the route was actually a device-inventory entry page.
- Later top-level product areas such as Wireless Sync, LCD / Media, Diagnostics, and Help / Docs had no dedicated navigation targets.
- Page headers existed, but the pages did not consistently follow a shared header plus summary plus work-area pattern.
- Runtime context was presented differently from page to page.
- Reusable UI parts existed in code, but they were stored in a flat component directory rather than a clearer design-system-oriented structure.
- The theme and surface styling were coherent enough for MVP work, but the token system was too shallow for later expansion and documentation.

## Key Audit Findings

1. The MVP routes already contain enough functional depth to justify a true app shell instead of a simple route list.
2. The design language had useful building blocks, but not yet a documented or categorized visual system.
3. The route model needed to reserve space for future workbenches before those feature phases landed, otherwise later additions would feel bolted on.
4. Responsive behavior existed, but the structure was still page-by-page rather than system-first.
