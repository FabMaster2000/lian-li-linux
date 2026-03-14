# Frontend State Model

## Decision
The frontend uses `Context + Hooks`.

No additional client-state library was introduced for Phase 10.

## Why
- The app is still small enough to avoid React Query, Zustand, or Redux.
- The live event channel is already global by nature and fits naturally into Context.
- Page-local editing state should stay local to the relevant workbench hook.

## Separation
- Server State:
  - canonical backend snapshots
  - loaded through `frontend/src/state/server/useServerResource.ts`
  - examples: dashboard snapshot, device detail snapshot, readonly lighting/fan state
- UI State:
  - route selection, form drafts, currently selected device, sliders, pending edits
  - kept inside page-specific hooks such as `frontend/src/hooks/useLightingWorkbenchData.ts` and `frontend/src/hooks/useFansWorkbenchData.ts`
- Live Events:
  - websocket transport, reconnect, and event fan-out
  - handled by `frontend/src/app/BackendEventsProvider.tsx`
  - consumed through `frontend/src/hooks/useBackendEventSubscription.ts`

## Applied Structure
- `frontend/src/state/server/useServerResource.ts`
  - shared async server-resource primitive with `loading`, `refreshing`, `error`, `lastUpdated`, and `refresh`
- `frontend/src/app/BackendEventsProvider.tsx`
  - shared live-event transport
- page hooks
  - compose server state and UI state instead of mixing both concerns into one opaque store

## Scope
Phase 10.2 intentionally does not build a full global entity store.

The current model keeps:
- global concerns global
- local editing concerns local
- server snapshots replaceable and event-refreshable

That keeps the architecture understandable and makes a later migration to React Query or Zustand possible if the app complexity actually grows.
