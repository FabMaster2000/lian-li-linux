# Backend Event Model

## Scope
This document describes the internal backend event abstraction introduced in Phase 7.2.
It is the source model that a later WebSocket endpoint can expose directly.

## Status
- implemented internally in the backend
- exposed publicly via `GET /api/ws`
- fed by polling and backend-side write paths

## Event Envelope

```json
{
  "type": "lighting.changed",
  "timestamp": "2026-03-14T10:15:30Z",
  "source": "api",
  "device_id": "wireless:02:11:22:33:44:55",
  "data": {
    "reason": "color_set",
    "zone": 0,
    "color": "#abcdef"
  }
}
```

Fields:
- `type`
  - normalized web event type
- `timestamp`
  - ISO-8601 UTC timestamp
- `source`
  - where the event came from, currently `poll` or `api`
- `device_id`
  - optional device scope
- `data`
  - event-specific payload

## Current Event Types

### Daemon lifecycle
- `daemon.connected`
- `daemon.disconnected`

### Device and telemetry
- `device.updated`
- `fan.changed`
- `openrgb.changed`

### Desired-state changes
- `lighting.changed`
- `config.changed`

## Current Sources

### Polling source
The backend runs an internal polling thread against the daemon:
- every 1 second:
  - `Ping`
  - `ListDevices`
  - `GetTelemetry`
- every 5th cycle, and also on reconnect/topology change:
  - `GetConfig`

The backend diffs snapshots and emits normalized events when changes are detected.
The detailed fallback strategy is documented in `docs/polling-fallback.md`.

### API-write source
Relevant backend write handlers emit immediate events after successful daemon writes:
- lighting endpoints emit `lighting.changed`
- fan manual endpoint emits `fan.changed`
- config replacement emits `config.changed`
- profile apply emits `config.changed` plus per-device lighting/fan events

## Design Notes
- polling events reflect observed daemon state
- API events reflect writes accepted by the backend
- the same event bus is intended to back a future WebSocket implementation
- no rollback or delivery persistence exists in this phase

## Summary
- the backend now has one normalized event envelope
- events can come from polling or local writes
- `GET /api/ws` now streams this envelope as WebSocket text frames
