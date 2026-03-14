# Frontend

The frontend in `frontend/` is a React + TypeScript + Vite application that consumes the backend API and WebSocket event stream. It is structured around typed services, page-level hooks, reusable UI primitives, and explicit separation between readonly server state and editable UI drafts.

## Main Structure

| Area | Responsibility | Key Paths |
| --- | --- | --- |
| `src/app` | app shell, providers, navigation, route setup | `frontend/src/app/App.tsx` |
| `src/pages` | route-level UI for dashboard, devices, lighting, fans, profiles, settings | `frontend/src/pages/` |
| `src/hooks` | page/workbench state, data loading, event subscriptions | `frontend/src/hooks/` |
| `src/services` | typed HTTP/WebSocket access to backend routes | `frontend/src/services/` |
| `src/state/server` | shared server-resource loading pattern | `frontend/src/state/server/useServerResource.ts` |
| `src/components` | reusable UI primitives and page sections | `frontend/src/components/` |
| `src/types` | mirrored API documents and frontend-only helper types | `frontend/src/types/` |
| `src/test` | test render helpers and setup | `frontend/src/test/` |

## Runtime Model

The frontend explicitly separates three kinds of state.

### Server state

Readonly backend snapshots loaded through services and stored via hooks such as:

- `useDashboardData`
- `useDeviceDetailData`
- `useLightingWorkbenchData`
- `useFansWorkbenchData`
- `useProfilesWorkbenchData`

Shared loading semantics live in `useServerResource`.

### UI state

Editable drafts such as:

- selected device
- selected zone
- current color/effect/brightness draft
- manual fan percent draft
- profile form fields

This state is local to the page/workbench hook and must not be overwritten by background refreshes.

### Live event state

`BackendEventsProvider` owns the WebSocket connection and publishes normalized backend events to interested hooks. Those hooks usually respond by triggering a background refresh of readonly data.

## Route Map

| Route | Purpose |
| --- | --- |
| `/` | dashboard and entry points |
| `/devices` | explicit device picker |
| `/devices/:deviceId` | per-device readonly detail view |
| `/lighting` | RGB workbench |
| `/fans` | manual fan workbench |
| `/profiles` | profile library and apply flow |
| `/settings` | runtime/diagnostics placeholder |

The browser UI intentionally starts with explicit device selection for device-oriented areas instead of auto-jumping into a default controller.

## Data Flow

### Read flow

```text
page
  -> hook
  -> frontend service
  -> backend API
  -> typed response
  -> readonly server state
```

### Write flow

```text
editable form draft
  -> service call
  -> backend response
  -> success or error state
  -> readonly state refresh or direct replacement
```

Current policy:

- no optimistic writes
- UI only commits after a successful backend response
- background refresh updates readonly state but keeps editable drafts intact

That policy is documented in `docs/frontend-state-strategy.md`.

### Live update flow

```text
/api/ws
  -> BackendEventsProvider
  -> useBackendEventSubscription
  -> page/workbench hook
  -> background refresh of readonly server state
```

## Reusable UI Conventions

Common controls already exist and should be reused before adding new one-off widgets:

- `StatusBadge`
- `ColorField`
- `SliderField`
- `EffectSelect`
- `DeviceCard`
- `DeviceOverviewCard`

This keeps lighting, fans, dashboard, and profiles visually and behaviorally aligned.

## Extending the Frontend

For a new feature, prefer this sequence:

1. extend `src/types/api.ts` if the backend contract changed
2. add or extend a service in `src/services/`
3. create a dedicated hook for the page/workbench logic
4. keep readonly server data and editable drafts separate
5. subscribe to backend events only if the feature needs live refresh
6. add page tests and, if useful, service or hook tests
7. update `frontend/README.md`

Avoid pushing fetch logic directly into page components.

## Testing

The frontend test suite currently covers:

- shared API client behavior
- system service delegation
- app shell and provider behavior
- page-level rendering and interaction
- workbench hooks for lighting, fans, profiles, dashboard, and device detail
- server-resource background refresh behavior

Run locally:

```bash
cd frontend
npm run test:run
npm run typecheck
npm run build
```

CI runs the same three validation steps.

## Known Limits

- The web UI currently focuses on dashboard, lighting, manual fans, profiles, and diagnostics.
- The settings route is still a placeholder for runtime and operational controls.
- Live updates are driven by backend polling/event synthesis, not direct daemon push messages.
- Advanced fan curves and richer LCD workflows are still future work.

## Related Docs

- `docs/api.md`
- `docs/frontend-state-model.md`
- `docs/frontend-state-strategy.md`
- `docs/event-model.md`
- `docs/container-tests.md`
