# Backend Architecture

This document describes the current backend architecture for the Lian Li Linux web controller.
It explains how the backend translates daemon-backed behavior into a browser-facing HTTP and WebSocket surface.

- Status: implemented
- Audience: contributors, maintainers, operators
- Source of truth: `backend/src/`
- Last reviewed: 2026-03-14
- Related documents:
  - `system-overview.md`
  - `daemon-integration.md`
  - `../architecture/source-of-truth.md`

## Role in the System

The backend is the web-facing control plane between browser clients and `lianli-daemon`.
It does not replace the daemon. It adapts daemon IPC into web-safe documents, validates inputs, stores profiles, and publishes normalized live events.

## Current Module Layout

| Module | Responsibility |
| --- | --- |
| `main` | load config, initialize logging, start the Axum server |
| `app` | assemble `AppState`, daemon client, profile store, and event polling |
| `routes` | define public and protected HTTP and WebSocket routes |
| `auth` | enforce optional basic, bearer, or reverse-proxy-backed auth |
| `config` | load backend runtime config from file and environment |
| `daemon` | send IPC requests over the Unix socket |
| `handlers` | map requests to daemon operations and backend storage |
| `models` | define browser-facing response and request documents |
| `events` | normalize backend-side events and polling-based changes |
| `storage` | persist profiles in a JSON-backed store |
| `errors` | map internal failures to API-safe error responses |
| `tests` | route-level and integration-style backend tests |

## Request Lifecycle

```text
HTTP request
  -> route match
  -> auth middleware when required
  -> handler validation and mapping
  -> daemon IPC and/or backend storage operation
  -> backend response document
  -> optional event publication
```

Typical request patterns:

- direct daemon passthrough with mapping, such as device inventory and daemon status
- daemon config read or write plus normalized response mapping
- profile storage CRUD in the backend JSON store
- profile application by orchestrating daemon-backed config writes

## Current Public Surface

The backend currently provides:

- public health and version endpoints
- protected runtime and daemon-status endpoints
- protected configuration read and write endpoints
- protected device inventory and device detail endpoints
- protected lighting state and lighting mutation endpoints
- protected fan state and manual fan write endpoints
- protected profile list, create, update, delete, and apply endpoints
- protected WebSocket events at `/api/ws`

## State and Persistence

The backend owns two important categories of state:

- runtime state such as auth mode, paths, and daemon socket configuration
- profile storage, which is separate from daemon config and persisted as a backend-managed JSON document

Readonly device and telemetry snapshots are still daemon-derived.
The backend should not invent device truth that diverges from the daemon.

## Eventing

The backend publishes normalized events for:

- configuration changes
- lighting changes
- fan changes
- profile application side effects

Current live updates combine backend write-path publication with polling-based refresh where native daemon push is incomplete.

## Design Constraints

- Do not move hardware communication into the backend request handlers.
- Reuse daemon capabilities before introducing backend-only abstractions.
- Keep browser-facing models stable even when daemon-level data is more irregular.
- Surface partial success, daemon failures, and validation errors explicitly.

## Operational Notes

- The backend is intended to run next to the daemon on Linux and talk to it over a Unix socket.
- Authentication is optional and configured at backend startup.
- Runtime diagnostics already expose backend and daemon paths, which the dashboard uses for operator visibility.
- The current backend is sufficient for MVP inventory, lighting, fans, profiles, and runtime visibility, but later phases expand orchestration, diagnostics, and capability-specific APIs.
