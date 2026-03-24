# Eventing Analysis

This document records the current eventing limits and why the web controller still needs fallback refresh behavior in some areas.

- Status: partial
- Audience: backend contributors, frontend contributors
- Source of truth: backend event model, backend websocket implementation, and frontend live-update hooks
- Last reviewed: 2026-03-15
- Related documents:
  - `technical/architecture/daemon-integration.md`
  - `technical/architecture/source-of-truth.md`

## Current Situation

The backend exposes live updates through `/api/ws`, but the daemon and backend stack do not yet provide a fully normalized push model for every mutable surface.
Some pages still rely on explicit refreshes or hybrid push-plus-fetch behavior.

## Why This Matters

Without a consistent eventing model:

- pages reload more state than they strictly need
- race conditions are easier to introduce when mutations and reads overlap
- frontend hooks must keep more defensive refresh logic
- backend responsibilities become harder to reason about

## Current Event Sources

The backend currently emits a small set of practical events for browser clients.
These events are useful, but they do not yet cover every meaningful state transition with consistent payload richness.

Observed eventing patterns in the current stack include:

- state-changed notifications after successful mutations
- reconnect-driven full refreshes in the frontend
- fallback polling or explicit reloads for pages that need authoritative readback

## Historical Client Behavior

The removed native client followed a simple polling model:

- polling every 2 seconds
- `Ping`
- `ListDevices`
- `GetTelemetry`

On reconnect or when device count changes, it additionally reloaded:

- `GetConfig`
- `GetRgbCapabilities`
- `ListDevices`

It did not use `Subscribe`.

## Current Gaps

The main gaps still worth tracking are:

- inconsistent payload granularity between mutation events
- missing canonical ownership for some event names and payload schemas
- lack of a single end-to-end eventing document that covers daemon origin to browser consumption
- page-specific fallback refresh logic that remains necessary for confidence

## Recommended Direction

The long-term target should be:

1. one canonical eventing model document
2. explicit event ownership per backend surface
3. consistent browser-facing payload contracts
4. narrower frontend refreshes after known event types
5. documented fallback behavior when event coverage is incomplete

Until then, frontend pages should continue to prefer safe refresh behavior over optimistic assumptions.
