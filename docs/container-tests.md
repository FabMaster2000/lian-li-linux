# Container Tests

## Goal
Run daemon tests, backend tests, and an end-to-end smoke test inside a fresh Docker container with simulated hardware.

## Test Layers

### 1. Daemon tests
Run with:
```bash
cargo test -p lianli-daemon -- --nocapture
```

What they cover:
- default daemon config path resolution from `XDG_CONFIG_HOME` and `HOME`
- IPC `Ping` request handling
- `GetConfig` behavior before config is loaded
- `Subscribe` contract currently returning the documented "not implemented" error
- config file persistence through `SetConfig`
- parent directory creation for config writes

Relevant code:
- `crates/lianli-daemon/src/main.rs`
- `crates/lianli-daemon/src/ipc_server.rs`

### 2. Backend tests
Run with:
```bash
cargo test -p lianli-backend -- --nocapture
```

What they cover:
- basic API coverage:
  - `/api/health`
  - `/api/version`
  - `/api/runtime` including resolved `config_path`, `profile_store_path`, configured `log_level`, and redacted auth state
  - `/api/daemon/status` against a mocked daemon socket
  - `/api/devices` and URL-encoded device IDs
- device write/read coverage:
  - lighting state readback
  - lighting color/effect/brightness writes
  - lighting validation for invalid brightness values
  - fan state readback
  - manual fan writes
  - fan validation for invalid percent and slot values
- WebSocket coverage:
  - event streaming from the backend event hub
  - client text frames being ignored without breaking the stream
  - auth protection for `/api/ws`
- auth coverage:
  - public health endpoint while auth is enabled
  - `basic` auth success, missing-header rejection, and wrong-credential rejection
  - `bearer` auth success and wrong-token rejection
  - `reverse_proxy` header success and missing-header rejection
- config coverage:
  - `GET /api/config` mapping from daemon `AppConfig` to the web config document
  - `GET /api/config` default-section mapping when daemon RGB/fan config is absent
  - `POST /api/config` mapping from the web config document back to daemon `SetConfig`
  - config validation for invalid fan slot definitions, invalid colors/effects/directions/scopes, invalid `hid_driver`, and duplicate LCD targets
  - relative LCD media/font paths resolving against the daemon config directory before validation
- daemon error coverage:
  - `502` mapping for daemon transport failures on `/api/devices`
  - `502` mapping for daemon `Error` responses on `/api/devices` and `/api/config`
  - `/api/daemon/status` returning `reachable: false` with transport errors instead of failing the route
- profile coverage:
  - profile CRUD storage in `profiles.json`
  - profile payload validation failures
  - profile apply orchestration into a single daemon `SetConfig` write
  - `POST /api/profiles/:id/apply` error handling for unknown target devices
- internal backend coverage:
  - backend event envelope helpers and poll-diff logic
  - fallback polling refresh policy and noise filtering
  - lighting write event emission and generic event envelope publishing
  - backend config-file loading plus env parsing defaults and overrides for host, port, log level, storage paths, and auth options
  - direct `ProfileStore` unit tests for missing-file, malformed JSON, duplicate IDs, missing IDs, and CRUD roundtrip behavior

Relevant code:
- `backend/src/tests.rs`
- `backend/src/events/mod.rs`
- `backend/src/storage/mod.rs`

### 3. Smoke test
Run with:
```bash
bash scripts/smoke-daemon-backend.sh /tmp/lianli-smoke-tests
```

What it does:
- builds `lianli-daemon` and `lianli-backend`
- starts both binaries locally
- enables simulated hardware with `LIANLI_SIM_WIRELESS=slinf:3`
- waits for the daemon socket and backend HTTP listener
- calls the main API endpoints
- opens a WebSocket client on `/api/ws`
- writes lighting state
- verifies the resulting `lighting.changed` event over WebSocket
- writes manual fan speed
- performs a config read and config roundtrip write
- stores logs and a JSON summary

Supporting smoke helper:
- `scripts/await-websocket-event.py` waits for a matching event and persists it to `smoke-websocket-event.json`

### 4. Frontend tests
Run with:
```bash
cd frontend
npm run test:run
```

What they cover:
- API-client URL resolution, JSON-body handling, structured API-error mapping, synthetic network failures, and WebSocket parse/error handling
- system-service delegation for `health`, `version`, `runtime`, `daemon status`, and backend event connections
- application-shell navigation, active-route highlighting, and live-event status rendering
- websocket connection, reconnect, and provider-level event dispatch
- API reachability and daemon reachability state
- global API, daemon, and websocket notices
- shared server-state loading, background refresh, and error handling
- dashboard snapshot loading, event-triggered background refresh, and combined-load error handling
- device-detail snapshot loading, route-device decoding, partial secondary-load failures, and event-triggered refresh
- reusable UI primitives:
  - `StatusBadge`
  - `ColorField`
  - `SliderField`
  - `EffectSelect`
  - `DeviceCard`
- component routing defaults and explicit override links in `DeviceOverviewCard`
- dashboard rendering and dashboard error states
- `/devices` picker success and error states
- `/lighting` explicit-selection behavior, lighting apply flows, websocket-triggered readonly refresh behavior, offline-device handling, and compact live-state rendering
- `/fans` explicit-selection behavior, manual apply flows, websocket-triggered readonly refresh behavior, offline-device handling, mixed slot states, and preserved RPM telemetry after apply
- `/profiles` rendering plus create, delete, apply, and target-validation flows
- device-detail offline-state rendering
- hook-level state management for lighting, fan, and profile workbenches
- confirmed-response write behavior: profile list mutations only happen after successful backend writes

Relevant code:
- `frontend/src/app/BackendEventsProvider.test.tsx`
- `frontend/src/app/AppLayout.test.tsx`
- `frontend/src/app/SystemStatusProvider.test.tsx`
- `frontend/src/components/ColorField.test.tsx`
- `frontend/src/components/DeviceCard.test.tsx`
- `frontend/src/components/DeviceOverviewCard.test.tsx`
- `frontend/src/components/EffectSelect.test.tsx`
- `frontend/src/components/GlobalSystemNotices.test.tsx`
- `frontend/src/components/SliderField.test.tsx`
- `frontend/src/components/StatusBadge.test.tsx`
- `frontend/src/hooks/useDashboardData.test.tsx`
- `frontend/src/hooks/useDeviceDetailData.test.tsx`
- `frontend/src/hooks/useProfilesWorkbenchData.test.tsx`
- `frontend/src/hooks/useLightingWorkbenchData.test.tsx`
- `frontend/src/hooks/useFansWorkbenchData.test.tsx`
- `frontend/src/pages/DashboardPage.test.tsx`
- `frontend/src/pages/DevicesPage.test.tsx`
- `frontend/src/pages/DeviceDetailPage.test.tsx`
- `frontend/src/pages/LightingPage.test.tsx`
- `frontend/src/pages/FansPage.test.tsx`
- `frontend/src/pages/ProfilesPage.test.tsx`
- `frontend/src/pages/SettingsPage.test.tsx`
- `frontend/src/services/api.test.ts`
- `frontend/src/services/system.test.ts`
- `frontend/src/state/server/useServerResource.test.tsx`

Optional local frontend validation:
```bash
cd frontend
npm run typecheck
npm run build
```

CI validation:
- `.github/workflows/rust.yml` now runs a parallel `frontend` job on `ubuntu-latest`
- the job executes `npm ci`, `npm run test:run`, `npm run typecheck`, and `npm run build`

## One-click usage

### PowerShell
```powershell
./scripts/run-container-tests.ps1
```

### Bash
```bash
./scripts/run-container-tests.sh
```

## What the one-click runner does
The one-click scripts:
- build a fresh test image from `docker/test-runner.Dockerfile`
- create a fresh container
- mount the current repo into the container at `/work`
- mount an artifact directory at `/artifacts`
- execute `scripts/run-test-suite.sh`

Inside the container, `scripts/run-test-suite.sh`:
- runs daemon unit tests
- runs backend unit tests
- stages the frontend into an isolated work directory under the artifact path
- installs frontend dependencies with `npm ci`
- runs frontend tests, typecheck, and production build
- runs the smoke test
- generates `container-tests-report.md` inside the mounted artifact directory

## Manual execution without Docker

### Linux or WSL
Run daemon tests:
```bash
cargo test -p lianli-daemon -- --nocapture
```

Run backend tests:
```bash
cargo test -p lianli-backend -- --nocapture
```

Run the end-to-end smoke test:
```bash
bash scripts/smoke-daemon-backend.sh /tmp/lianli-smoke-tests
```

Requirements for the manual smoke test:
- Rust toolchain installed
- `bash`, `curl`, and `jq` available
- `python3` with the `websockets` module available
- Linux or WSL environment with Unix socket support

## Artifacts
Default output directory:
```text
artifacts/container-tests/
```

Important files:
- `cargo-test-daemon.log`
- `cargo-test-backend.log`
- `npm-ci-frontend.log`
- `npm-test-frontend.log`
- `npm-typecheck-frontend.log`
- `npm-build-frontend.log`
- `cargo-build-smoke.log`
- `smoke-daemon.log`
- `smoke-backend.log`
- `smoke-websocket.log`
- `smoke-websocket-event.json`
- `smoke-summary.json`
- `container-tests-report.md`

What they contain:
- `cargo-test-daemon.log`: output from daemon unit tests
- `cargo-test-backend.log`: output from backend unit tests
- `npm-ci-frontend.log`: frontend dependency installation log from the container run
- `npm-test-frontend.log`: frontend Vitest output
- `npm-typecheck-frontend.log`: frontend TypeScript validation output
- `npm-build-frontend.log`: frontend production build output
- `cargo-build-smoke.log`: build log used by the smoke test
- `smoke-daemon.log`: daemon runtime log during the smoke run
- `smoke-backend.log`: backend runtime log during the smoke run
- `smoke-websocket.log`: output from the WebSocket smoke helper
- `smoke-websocket-event.json`: captured `lighting.changed` event from `/api/ws`
- `smoke-summary.json`: structured result of the API checks
- `container-tests-report.md`: combined markdown report with summary and all logs

## Requirements
- Docker must be installed and reachable from the host shell for one-click execution.
- The host repo is mounted into `/work` inside the test container.

## Notes
- The smoke test uses the daemon's built-in simulated wireless mode and does not require physical hardware.
- The smoke test exercises:
  - `/api/health`
  - `/api/daemon/status`
  - `/api/devices`
  - `/api/devices/:id/lighting`
  - `/api/devices/:id/lighting/color`
  - `/api/devices/:id/fans`
  - `/api/devices/:id/fans/manual`
  - `/api/config`
  - `/api/ws`
- The smoke test writes state, so it creates a temporary config under the chosen artifact/config directory.
