# Backend

`backend/` is the HTTP and WebSocket bridge between browser clients and `lianli-daemon`. It adapts daemon IPC data into web-friendly documents, enforces request validation, stores profiles, and emits normalized live events.

## Module Overview

| Module | Responsibility | Key File |
| --- | --- | --- |
| `main` | load config, initialize logging, start Axum server | `backend/src/main.rs` |
| `app` | assemble `AppState`, daemon client, profile store, and event polling | `backend/src/app.rs` |
| `routes` | define public/protected routes and auth middleware placement | `backend/src/routes/mod.rs` |
| `auth` | optional `basic`, `bearer`, and `reverse_proxy` auth checks | `backend/src/auth/mod.rs` |
| `config` | backend startup config loading from file and env | `backend/src/config/mod.rs` |
| `daemon` | Unix-socket IPC client for daemon requests | `backend/src/daemon/mod.rs` |
| `handlers` | route handlers plus mapping and validation helpers | `backend/src/handlers/mod.rs` |
| `models` | HTTP documents and runtime payload models | `backend/src/models/mod.rs` |
| `events` | event hub, polling diff logic, and backend event publication | `backend/src/events/mod.rs` |
| `storage` | JSON-backed profile persistence | `backend/src/storage/mod.rs` |
| `errors` | shared API error mapping to HTTP responses | `backend/src/errors/mod.rs` |
| `tests` | route-level and integration-style tests with mock daemon sockets | `backend/src/tests.rs` |

## Startup Sequence

```text
main
  -> AppConfig::load()
  -> tracing init
  -> app::run(config)
  -> DaemonClient + ProfileStore + EventHub
  -> events::start_polling(...)
  -> Axum router
  -> bind and serve
```

The startup contract matters because:

- auth mode is loaded once at startup
- profile store path is fixed at startup
- the polling event loop starts once with the daemon socket path from config

## Route Structure

Public routes:

- `GET /api/health`
- `GET /api/version`

Protected routes:

- runtime and daemon diagnostics
- devices and per-device operations
- config API
- profiles API
- WebSocket events

Auth is applied at the router boundary in `backend/src/routes/mod.rs`, not scattered inside handlers.

## Request Handling Pattern

Most handlers follow the same pattern:

1. validate request and route parameters
2. load daemon state if needed
3. translate to or from daemon/shared types
4. send IPC request or update profile store
5. publish a backend event if the write succeeded
6. return a web-facing document

That pattern is visible in:

- lighting writes in `backend/src/handlers/mod.rs`
- fan writes in `backend/src/handlers/mod.rs`
- config reads/writes in `backend/src/handlers/mod.rs`
- profile CRUD/apply in `backend/src/handlers/mod.rs`

## Config Handling

The backend exposes a web-facing config document instead of forwarding the daemon `AppConfig` directly.

Reasons:

- the daemon types are hardware- and implementation-oriented
- the web API needs stable strings and explicit documents
- validation messages need to be useful to browser clients

Important behavior:

- `GET /api/config` maps daemon config into `ConfigDocument`
- `POST /api/config` validates and maps back into daemon config, then writes once with `SetConfig`
- relative LCD media and sensor-font paths are validated relative to the daemon config directory
- the daemon still remains the owner of the actual persisted hardware config

## Profiles

Profiles are intentionally not part of daemon config.

Current model:

- backend stores profile documents in `profiles.json`
- profile apply reads current daemon config
- the backend merges the selected profile into that config
- the backend performs one daemon `SetConfig` write
- apply returns which sections/devices were applied or skipped

See also:

- `docs/profile-model.md`
- `docs/profile-storage.md`

## Eventing

The backend event system exists to give browser clients a live stream even though daemon subscriptions are not implemented yet.

Sources of events:

- polling diffs from daemon snapshots
- successful API write operations

Transport:

- internal `EventHub`
- WebSocket stream at `/api/ws`

The normalized envelope is documented in `docs/event-model.md`.

## Error Model

Backend errors are intentionally grouped into a small set:

- `BAD_REQUEST`
- `UNAUTHORIZED`
- `NOT_FOUND`
- `DAEMON_ERROR`
- `INTERNAL_ERROR`

Important behavior:

- daemon transport and daemon message errors both map to `502`
- auth failures may include `WWW-Authenticate`
- `/api/daemon/status` is a diagnostic endpoint and reports daemon failures in the JSON body instead of failing the route

## Extending the Backend

If you add a new feature, use this sequence:

1. define or reuse API models in `backend/src/models/mod.rs`
2. add handler logic in `backend/src/handlers/mod.rs`
3. register the route in `backend/src/routes/mod.rs`
4. emit a normalized event if browser clients should update live
5. add route-level tests in `backend/src/tests.rs`
6. update `docs/api.md`

Prefer route tests over isolated helper exposure. The existing test suite already uses mock daemon sockets for this purpose.

## Known Limits

- There is no server-side database; profiles use JSON storage only.
- Auth changes require a restart because config is startup-bound.
- Backend eventing is a polling fallback, not a direct daemon push channel.
- The backend should not grow hardware-specific logic that belongs in the daemon or shared crates.

## Related Docs

- `docs/api.md`
- `docs/backend-config.md`
- `docs/deployment.md`
- `docs/event-model.md`
- `docs/polling-fallback.md`
- `docs/profile-model.md`
- `docs/profile-storage.md`
- `docs/container-tests.md`
