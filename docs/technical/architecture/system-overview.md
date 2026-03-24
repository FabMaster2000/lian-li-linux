# System Overview

This document describes the current end-to-end structure of the Lian Li Linux web controller.
It is the technical starting point for understanding how the browser UI, web backend, daemon, and hardware layers work together in the current MVP-plus state.

- Status: implemented
- Audience: contributors, maintainers, operators
- Source of truth: current frontend, backend, daemon, and shared-type implementation
- Last reviewed: 2026-03-14
- Related documents:
  - `backend-architecture.md`
  - `frontend-architecture.md`
  - `daemon-integration.md`
  - `domain-model.md`

## System At a Glance

```text
Browser UI
  -> frontend/ React application
  -> HTTP + WebSocket
  -> lianli-backend
  -> Unix socket IPC
  -> lianli-daemon
  -> lianli-transport + lianli-devices
  -> Lian Li hardware
```

The web stack is intentionally daemon-backed.
The frontend never talks to hardware directly and should not duplicate transport or controller logic that already exists below the backend.

## Current Runtime Components

- `frontend/`: React + TypeScript + Vite application served in the browser
- `backend/`: Axum-based HTTP and WebSocket bridge between the browser and the daemon
- `crates/lianli-daemon/`: process that owns IPC, config application, and hardware-facing control workflows
- `crates/lianli-shared/`: shared IPC, config, fan, RGB, media, and device-related types
- `crates/lianli-transport/`: low-level transport helpers
- `crates/lianli-devices/`: device detection and device-family-specific behavior

## Current Web Product Surface

The current frontend exposes these routes:

- `/`: dashboard
- `/devices`: device selection page
- `/devices/:deviceId`: device detail
- `/lighting`: single-device lighting workbench
- `/fans`: single-device manual fan control
- `/profiles`: profile library and application
- `/settings`: operational placeholder for runtime and diagnostics expansion

The current backend exposes:

- health and version endpoints
- runtime and daemon status endpoints
- configuration read and write endpoints
- device inventory and device detail endpoints
- lighting read and write endpoints
- fan read and manual write endpoints
- profile CRUD and profile apply endpoints
- a WebSocket event stream

## Ownership Boundaries

- The daemon owns transport behavior, device discovery, low-level capability handling, and config application.
- The backend owns web-safe request validation, HTTP and WebSocket contracts, profile persistence, and event normalization.
- The frontend owns user workflows, route structure, draft form state, and presentation of backend-confirmed state.
- Functional documentation owns user-facing workflow descriptions.
- Technical documentation owns architecture, contracts, and operational guidance.

## Current Strengths

- Hardware access already stays behind the daemon boundary.
- The backend exposes a coherent browser-facing API surface for the current MVP routes.
- The frontend already separates server state from editable draft state on the main workbench pages.
- The project already has a working device inventory, device detail, lighting, fans, profiles, and settings route map.

## Current Gaps

- The technical documentation is still being migrated out of the legacy flat `docs/` tree.
- Functional documentation is only now being established as a separate top-level documentation area.
- Dashboard, lighting, fans, and profiles are usable MVP surfaces, but richer workbench and orchestration features belong to later phases.
- Settings is currently a placeholder route rather than a full diagnostics and operations workspace.

## Reading Order

Use this order when onboarding into the system:

1. This document for the big-picture structure
2. `backend-architecture.md` for backend ownership and request flow
3. `frontend-architecture.md` for route and state structure
4. `daemon-integration.md` for daemon-boundary rules and IPC behavior
5. `domain-model.md` for the web-facing model of devices, capabilities, and profiles
