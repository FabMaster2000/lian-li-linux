# Architecture

This document is the developer entry point for the current stack. It explains how the daemon, backend, frontend, CLI, and desktop GUI fit together, where state lives, and which extension points already exist.

## Reading Order

Use this order if you are new to the repository:

1. `README.md`
2. `docs/architecture.md`
3. `docs/backend.md`
4. `docs/frontend.md`
5. `docs/api.md`

Then go deeper into the topic-specific docs linked at the end of this file.

## Runtime Topology

```text
Browser
  -> frontend/ React app
  -> HTTP + WebSocket
  -> lianli-backend
  -> Unix socket IPC
  -> lianli-daemon
  -> lianli-devices / lianli-transport
  -> Lian Li hardware

Desktop GUI
  -> lianli-gui
  -> Unix socket IPC
  -> lianli-daemon

CLI
  -> tools/lianli-cli
  -> Unix socket IPC
  -> lianli-daemon
```

Two important consequences follow from this layout:

- The daemon remains the single authority for direct hardware access and persisted device config.
- The backend is intentionally a translation and orchestration layer, not a second hardware controller.

## Workspace Modules

| Module | Purpose | Key Paths |
| --- | --- | --- |
| `crates/lianli-daemon` | Headless hardware runtime, fan loop, LCD/media streaming, IPC server | `crates/lianli-daemon/src/main.rs`, `crates/lianli-daemon/src/ipc_server.rs` |
| `crates/lianli-devices` | Device-specific drivers and detection | `crates/lianli-devices/src/lib.rs`, `crates/lianli-devices/src/detect.rs` |
| `crates/lianli-transport` | HID/USB transport abstractions | `crates/lianli-transport/src/lib.rs` |
| `crates/lianli-media` | Sensor rendering plus image/video/GIF preparation | `crates/lianli-media/src/lib.rs` |
| `crates/lianli-shared` | Shared config model, IPC schema, RGB/fan/media types | `crates/lianli-shared/src/ipc.rs`, `crates/lianli-shared/src/config.rs` |
| `crates/lianli-gui` | Existing Slint desktop client | `crates/lianli-gui/src/main.rs`, `crates/lianli-gui/src/backend.rs` |
| `tools/lianli-cli` | CLI client reusing daemon IPC | `tools/lianli-cli/src/main.rs` |
| `backend` | HTTP/WebSocket bridge for browser clients | `backend/src/main.rs`, `backend/src/routes/mod.rs` |
| `frontend` | React/Vite browser UI | `frontend/src/app/App.tsx`, `frontend/src/pages/` |

## State Ownership

The project deliberately separates state by responsibility:

- Hardware state:
  - owned by `lianli-daemon`
  - queried or changed through IPC
- Persistent device config:
  - owned by the daemon config JSON
  - written by the daemon or through backend requests that end in daemon `SetConfig`
- Web/backend profile library:
  - owned by `backend`
  - stored in `profiles.json`
- Frontend UI draft state:
  - owned by the active React page/hook
  - not persisted until a write succeeds

This is why the frontend can refresh readonly data without overwriting in-progress edits.

## Main Request Flows

### Browser read flow

```text
Frontend page/hook
  -> frontend service
  -> /api/*
  -> backend handler
  -> daemon IPC request
  -> backend response mapping
  -> typed frontend state
```

Examples:

- Dashboard:
  - `GET /api/devices`
  - `GET /api/daemon/status`
  - `GET /api/runtime`
- Device detail:
  - `GET /api/devices/:id`
  - `GET /api/devices/:id/lighting`
  - `GET /api/devices/:id/fans`

### Browser write flow

```text
Editable frontend form
  -> POST /api/devices/:id/... or /api/config or /api/profiles
  -> backend validation
  -> daemon IPC write or profile-store write
  -> backend publishes normalized event
  -> frontend refreshes readonly server state
```

Examples:

- lighting writes use `SetRgbConfig`
- fan manual writes use `SetFanConfig`
- config writes use `SetConfig`
- profile writes stay inside the backend JSON store

### Event flow

The daemon does not currently provide a native push subscription channel. The backend therefore normalizes events in two ways:

- polling-based change detection against daemon snapshots
- immediate backend-generated events after successful API writes

Those events are published through `EventHub` and streamed to browser clients over `/api/ws`.

## Request-Flow Rules

There are a few architectural rules worth keeping:

- Do not let the frontend write daemon JSON directly.
- Do not introduce hardware access into the backend.
- Prefer adapting daemon state into web-friendly documents in the backend model layer.
- Keep profile storage separate from daemon config storage.
- Treat WebSocket events as a transport over normalized backend events, not as raw daemon traffic.

## Key Design Decisions

### Backend stays stateless about hardware

The backend caches only what is needed for polling diffs and request handling. It does not become a second source of truth for device state.

### Polling fallback is intentional

Until the daemon has a real subscription mechanism, the backend owns the event bridge. The polling policy is documented in `docs/polling-fallback.md`.

### Auth is optional and startup-bound

Backend auth is configured through `backend.json` and only applied at process start. A restart is enough to enable or disable it. No live auth reload is implemented.

### Profiles are a backend concern

Profiles are user-facing compositions of lighting/fan intent. They are not stored inside the daemon config.

## Known Limits

- Daemon push subscriptions are not implemented; backend eventing is polling-based.
- The web UI currently focuses on lighting, manual fan control, profiles, and diagnostics.
- The profile store is a simple JSON file and is not designed for multi-writer backend deployments.
- Auth is optional, but changing auth mode or credentials requires a backend restart.
- Fan curves, richer LCD workflows, and multi-device orchestration are intentionally deferred.

## Related Docs

- `docs/backend.md`
- `docs/frontend.md`
- `docs/api.md`
- `docs/backend-config.md`
- `docs/deployment.md`
- `docs/event-model.md`
- `docs/future-extensions.md`
- `docs/polling-fallback.md`
- `docs/profile-model.md`
- `docs/profile-storage.md`
- `docs/container-tests.md`
