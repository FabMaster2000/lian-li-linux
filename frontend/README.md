# Frontend

Minimal React + TypeScript + Vite scaffold for the web UI.

## Development

Install dependencies:

```bash
npm install
```

Start the dev server:

```bash
npm run dev
```

Build for production:

```bash
npm run build
```

Type-check the project:

```bash
npm run typecheck
```

Run the frontend test suite:

```bash
npm test
```

Run the frontend tests once for CI/local verification:

```bash
npm run test:run
```

List the current frontend test files:

```bash
rg --files src -g "*.test.ts" -g "*.test.tsx"
```

## Notes

- The Vite dev server proxies `/api` to `http://127.0.0.1:9000`.
- The app now uses `react-router-dom` for the base layout and route navigation.
- The scaffold already contains the target folder layout for later tasks:
  - `src/app`
  - `src/components`
  - `src/pages`
  - `src/features`
  - `src/services`
  - `src/hooks`
  - `src/types`
  - `src/styles`
- The typed API client layer lives in:
  - `src/services/api.ts`
  - `src/services/system.ts`
  - `src/services/devices.ts`
  - `src/services/lighting.ts`
  - `src/services/fans.ts`
  - `src/services/config.ts`
  - `src/services/profiles.ts`
- Shared request/response types are mirrored from the backend in `src/types/api.ts`.
- Frontend CI now runs in `.github/workflows/rust.yml` with:
  - `npm ci`
  - `npm run test:run`
  - `npm run typecheck`
  - `npm run build`
- Current base routes:
  - `/`
  - `/devices`
  - `/devices/:deviceId`
  - `/lighting`
  - `/fans`
  - `/profiles`
  - `/settings`
- Device-oriented areas now require explicit selection first:
  - `/devices` lists available devices before opening `/devices/:deviceId`
  - `/lighting` starts without a preselected device unless `?device=` is explicitly present
  - `/fans` starts without a preselected device unless `?device=` is explicitly present
- The dashboard route now loads live data from:
  - `GET /api/devices`
  - `GET /api/daemon/status`
  - `GET /api/runtime`
- The device detail route now loads live data from:
  - `GET /api/devices/:id`
  - `GET /api/devices/:id/lighting`
  - `GET /api/devices/:id/fans`
- The lighting route now uses:
  - `GET /api/devices`
  - `GET /api/devices/:id/lighting`
  - `POST /api/devices/:id/lighting/effect`
- The fan route now uses:
  - `GET /api/devices`
  - `GET /api/devices/:id/fans`
  - `POST /api/devices/:id/fans/manual`
- The profile route now uses:
  - `GET /api/devices`
  - `GET /api/profiles`
  - `POST /api/profiles`
  - `DELETE /api/profiles/:id`
  - `POST /api/profiles/:id/apply`
- Live backend events now use:
  - `GET /api/ws` via the shared `BackendEventsProvider`
  - automatic reconnect after socket loss
  - event dispatch into frontend hooks for dashboard, lighting, fans, and device detail refreshes
- The chosen state model for Phase 10 is documented in `docs/frontend-state-model.md`.
- The frontend now separates:
  - server state through shared server-resource hooks
  - UI state inside page/workbench hooks
  - live events through the websocket provider
- Global runtime robustness now includes:
  - API reachability checks
  - daemon reachability checks
  - websocket disconnect/reconnect notices
  - offline-device warnings on detail and workbench pages
- Shared UI primitives now live in:
  - `src/components/DeviceCard.tsx`
  - `src/components/StatusBadge.tsx`
  - `src/components/ColorField.tsx`
  - `src/components/SliderField.tsx`
  - `src/components/EffectSelect.tsx`
  - `src/components/SectionCard.tsx`
- Frontend write behavior is documented in `docs/frontend-state-strategy.md`.
- The chosen strategy is:
  - no optimistic writes
  - local state changes only after a successful backend response
  - readonly refreshes must not overwrite editable drafts
- The frontend test suite currently contains these files:
  - `src/app/AppLayout.test.tsx`
  - `src/app/BackendEventsProvider.test.tsx`
  - `src/app/SystemStatusProvider.test.tsx`
  - `src/components/ColorField.test.tsx`
  - `src/components/DeviceCard.test.tsx`
  - `src/components/DeviceOverviewCard.test.tsx`
  - `src/components/EffectSelect.test.tsx`
  - `src/components/GlobalSystemNotices.test.tsx`
  - `src/components/SliderField.test.tsx`
  - `src/components/StatusBadge.test.tsx`
  - `src/hooks/useDashboardData.test.tsx`
  - `src/hooks/useDeviceDetailData.test.tsx`
  - `src/hooks/useProfilesWorkbenchData.test.tsx`
  - `src/hooks/useLightingWorkbenchData.test.tsx`
  - `src/hooks/useFansWorkbenchData.test.tsx`
  - `src/pages/SettingsPage.test.tsx`
  - `src/pages/DashboardPage.test.tsx`
  - `src/pages/DevicesPage.test.tsx`
  - `src/pages/DeviceDetailPage.test.tsx`
  - `src/pages/LightingPage.test.tsx`
  - `src/pages/FansPage.test.tsx`
  - `src/pages/ProfilesPage.test.tsx`
  - `src/services/api.test.ts`
  - `src/services/system.test.ts`
  - `src/state/server/useServerResource.test.tsx`
- The list above is synchronized with `rg --files src -g "*.test.ts" -g "*.test.tsx"` and should be updated whenever a new frontend test file is added.
- The frontend tests currently cover:
  - API client URL building, JSON body handling, structured API errors, synthetic network errors, and WebSocket parse/error handling
  - system-service delegation for `health`, `version`, `runtime`, `daemon status`, and event connections
  - application shell navigation, active route highlighting, and live-event status rendering
  - websocket connection, reconnect, and event dispatch through `BackendEventsProvider`
  - API reachability and daemon reachability state through `SystemStatusProvider`
  - global API, daemon, and websocket notices through `GlobalSystemNotices`
  - shared server-state loading, background refresh, and error handling through `useServerResource`
  - dashboard snapshot loading, event-driven background refresh, and combined-load failures through `useDashboardData`
  - device-detail snapshot loading, route-device decoding, partial secondary-load failures, and event-driven refresh through `useDeviceDetailData`
  - reusable component rendering and change handling through `StatusBadge`, `ColorField`, `SliderField`, and `EffectSelect`
  - shared device-card rendering for status, capabilities, telemetry, and action links
  - component navigation defaults and explicit link overrides for `DeviceOverviewCard`
  - dashboard metrics, generic section entry links, and dashboard error states
  - device picker success and error states on `/devices`
  - settings placeholder routing and document-title behavior on `/settings`
  - explicit device selection before opening `/devices/:deviceId`, `/lighting`, and `/fans`
  - lighting device loading, zone switching, apply success, apply failure, readonly refresh behavior, websocket-driven readonly refresh, offline-device handling, and compact live-state rendering
  - fan device loading, manual apply success, apply failure, mixed slot state rendering, readonly refresh behavior, websocket-driven readonly refresh, offline-device handling, and preserved RPM telemetry after apply
  - profile listing, create/delete/apply flows, and explicit-device validation on `/profiles`
  - device detail offline-state rendering
  - hook-level state management for lighting, fan, and profile workbenches, including background refresh behavior that does not overwrite editable drafts and non-optimistic profile mutations
- The lighting-specific tests verify:
  - `src/hooks/useLightingWorkbenchData.test.tsx`
  - initial RGB device and zone loading
  - explicit selection on the generic `/lighting` route
  - zone switching updates the form state
  - applying lighting changes sends the correct payload
  - backend failures are surfaced in the UI
- The fan-specific tests verify:
  - `src/hooks/useFansWorkbenchData.test.tsx`
  - `src/pages/FansPage.test.tsx`
  - explicit selection on the generic `/fans` route
  - initial fan-device and slot loading
  - applying a manual all-slot percentage
  - mixed-value rendering when backend slot values differ
  - previous RPM telemetry is preserved when the update response omits `rpms`
  - backend failures are surfaced in the UI
- The profile-specific tests verify:
  - `src/hooks/useProfilesWorkbenchData.test.tsx`
  - `src/pages/ProfilesPage.test.tsx`
  - stored profiles are rendered from backend data
  - profiles can be created, deleted, and applied from the browser
  - explicit target-device validation blocks invalid profile creation
  - profile list mutations happen only after a successful backend response
