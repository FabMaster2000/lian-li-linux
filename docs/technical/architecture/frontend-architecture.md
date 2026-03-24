# Frontend Architecture

This document describes the current frontend architecture of the Lian Li Linux web controller.
It focuses on route structure, shared providers, state boundaries, and the current page model of the React application.

- Status: implemented
- Audience: contributors, maintainers
- Source of truth: `frontend/src/`
- Last reviewed: 2026-03-21
- Related documents:
  - `system-overview.md`
  - `../architecture/source-of-truth.md`
  - `../development/documentation-style-guide.md`

## Role in the System

The frontend is a browser-delivered React application.
It is responsible for routing, interactive workbench flows, user-visible state, and explicit device selection, while the backend remains the source of live control data.

## Current Route Map

| Route | Page | Current purpose |
| --- | --- | --- |
| `/` | Dashboard | summarize system status, device inventory, and route entry points |
| `/devices` | Devices | device inventory with search and family filtering |
| `/devices/:deviceId` | Device Detail | inspect device identity, capabilities, and current state |
| `/rgb` | Lighting | adjust lighting per cluster or across all paired devices |
| `/fans` | Fans | manual speed or curve mode per cluster with batch apply |
| `/wireless-sync` | Wireless Sync | pair and manage wireless clusters |
| `/profiles` | Profiles | create, store, apply, and remove reusable presets |
| `/diagnostics` | Diagnostics | runtime health: daemon, API, and event stream status |
| `/settings` | Settings | system configuration and hardware tool links |
| `/help` | Help / Docs | tabbed documentation navigator |
| `/lcd-media` | LCD / Media | placeholder redirect (not yet implemented) |

## Shared Application Structure

- `src/app/`: router, app layout, providers, navigation
- `src/pages/`: route-level pages
- `src/hooks/`: page and workbench data loading plus event-aware state
- `src/services/`: typed HTTP and WebSocket service calls
- `src/components/`: reusable UI pieces such as cards, badges, form inputs, and page intros
- `src/state/server/`: shared helper for server-resource loading patterns
- `src/types/`: mirrored API-facing frontend types

## Current State Model

The frontend distinguishes three practical state categories:

- readonly backend snapshots loaded from API services
- editable draft state for workbench forms
- app-wide runtime awareness such as backend events and system status

Important current behavior:

- dashboard and detail pages refresh backend-derived state
- lighting and fan pages refresh readonly state without discarding in-progress edits
- profiles manage a local draft plus backend-stored profile library state

## Current Navigation Model

The primary navigation exposes nine product areas:

- Dashboard
- Devices (with nested device detail)
- Lighting
- Fans
- Wireless Sync
- Profiles
- Diagnostics
- Settings
- Help / Docs

LCD / Media is the only remaining placeholder redirect.

## Current UX Characteristics

- page intros summarize the route purpose and current runtime context
- dashboard shows live inventory and daemon/runtime visibility
- workbenches use success, warning, and error banners explicitly
- lighting and fans support reload plus apply flows
- profiles already combine stored presets with live device inventory targeting

## Current Gaps

- LCD / Media is the only product area still behind a placeholder redirect
- Diagnostics is implemented but scaffold-level with limited operational depth
- design-system documentation exists but the component catalog is not yet exhaustive
- settings content is broad and remains partially placeholder for later operational work

## Frontend Rules

- The frontend must never talk directly to hardware or daemon transport layers.
- Route-level behavior should map to backend-confirmed state, not optimistic local assumptions.
- Capability gaps must be visible in the UI instead of hidden behind silent no-op behavior.
- New pages should follow the separated technical and functional documentation model introduced in the documentation overhaul.
