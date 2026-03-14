# Future Extensions

This document lists plausible follow-up work beyond the current MVP. It is intentionally separate from the current architecture and API docs so future ideas do not get mixed with already implemented behavior.

## Purpose

Use this file for:

- planning work after the current MVP
- keeping deferred features visible
- clarifying which ideas require daemon work, backend work, frontend work, or all three

This is not a commitment document. It is a technical roadmap of reasonable next extensions.

## Near-Term Extensions

### 1. Fan curves in the web UI

Current state:

- the daemon already supports fan curves
- the web UI currently focuses on manual fan percentages

Future work:

- curve CRUD in the browser
- curve editor UI with validation and preview
- assign curves per device or slot through the backend config API

Main areas:

- `backend/src/handlers/mod.rs`
- `backend/src/models/mod.rs`
- `frontend/src/pages/FansPage.tsx`
- `frontend/src/hooks/useFansWorkbenchData.ts`

Risk:

- curve editing needs careful UX to avoid invalid thermal control setups

### 2. Multi-device sync

Current state:

- profiles can target multiple devices during apply
- live workbenches are still largely single-device oriented

Future work:

- group selection in lighting and fans
- shared apply operations across selected devices
- synchronized RGB effects and brightness changes

Main areas:

- backend device-target orchestration
- frontend multi-select and conflict handling

Risk:

- device capability mismatches need explicit skip and conflict reporting

## Medium-Term Extensions

### 3. LCD, GIF, image, and video workflows in the browser

Current state:

- the daemon supports LCD media and sensor descriptors
- the web UI does not expose these workflows yet

Future work:

- LCD device picker and media assignment
- file upload or path-based media selection
- GIF/video preview and validation
- sensor-gauge editor in the browser

Main areas:

- backend config API reuse
- possibly additional upload/storage endpoints
- new frontend LCD workbench

Risk:

- browser upload flow, file lifecycle, and ffmpeg-dependent validation need clear operational rules

### 4. Richer profile model

Current state:

- profiles currently focus on lighting and manual fan behavior

Future work:

- include LCD presets
- include fan-curve references
- add profile tags, categories, import/export, and conflict metadata

Main areas:

- `docs/profile-model.md`
- backend profile storage and apply logic
- frontend profiles page

## Integration Extensions

### 5. MQTT and Home Assistant integration

Current state:

- no external automation integration exists

Future work:

- publish telemetry to MQTT
- subscribe to automation commands
- provide Home Assistant discovery payloads

Recommended approach:

- keep integration optional
- place it behind a separate integration module or companion service
- avoid coupling Home Assistant specifics into the core daemon

Risk:

- once external automation exists, auth, rate limiting, and observability become more important

### 6. CLI and automation growth

Current state:

- `lianli-cli` already reuses daemon IPC

Future work:

- broader scripting commands for config, profiles, and diagnostics
- machine-friendly output modes
- export/import helpers for profiles and config snapshots

## Security and Access Extensions

### 7. Roles and permissions

Current state:

- backend auth is optional and coarse-grained
- there is no role separation

Future work:

- read-only versus operator versus admin roles
- route-level write protection
- audit-friendly write attribution

Risk:

- role support should not be added before the access model is explicit and testable

### 8. User management

Current state:

- no user store exists
- auth supports static startup config only

Future work:

- local user database or delegated identity provider
- password reset and credential rotation flows
- user-scoped preferences or profile ownership

Recommended approach:

- prefer reverse-proxy or external identity integration before building a full local user system

## UX Extensions

### 9. Cloudless mobile-friendly PWA

Current state:

- the web UI is browser-based and responsive enough for desktop/mobile layouts
- it is not a PWA

Future work:

- installable local-network PWA
- offline shell for diagnostics and cached views
- touch-optimized workbench flows

Risk:

- offline behavior must not imply offline hardware control unless the backend is reachable

## Suggested Order

If the goal is a practical next roadmap, this order is the most coherent:

1. web fan curves
2. LCD/media browser workflows
3. multi-device sync
4. richer profiles
5. MQTT/Home Assistant integration
6. roles and permissions
7. user management
8. PWA packaging

## Explicit Non-Goals For The Current MVP

The current MVP intentionally does not try to solve:

- full automation platform integration
- multi-user identity management
- cloud features
- full LCD authoring in the browser
- advanced thermal-control editing UX

Keeping those out of the MVP reduces risk and keeps the current architecture understandable.
