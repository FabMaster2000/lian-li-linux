# Polling Fallback

## Scope
This document describes the backend-internal fallback polling strategy used in Phase 7.4.
It exists because the daemon still has no native `Subscribe` event stream.

## Goal
Provide live-ish updates without:
- hammering the daemon with unnecessary requests
- forwarding unchanged state repeatedly
- mixing high-frequency telemetry with broader device-change events

## Current Polling Strategy

### Fast cycle
Every 1 second the backend polls:
- `Ping`
- `ListDevices`
- `GetTelemetry`

Purpose:
- detect daemon availability
- detect attached/removed devices
- capture fan RPM and coolant telemetry
- capture OpenRGB runtime status

### Slower config cycle
The backend fetches `GetConfig`:
- every 5th polling cycle
- immediately after reconnect
- immediately after device topology changes

Purpose:
- detect persisted daemon config changes
- avoid a full config read on every fast tick

## Change Filtering

### Emitted only when changed
The backend compares the current snapshot with the previous snapshot and only emits events for
real changes.

Current behavior:
- `daemon.connected`
  - emitted once when the daemon becomes reachable
- `daemon.disconnected`
  - emitted once when the daemon becomes unreachable
- `device.updated`
  - emitted for device discovery/removal
  - emitted for device metadata or coolant-temperature changes
  - not emitted for fan RPM-only changes
- `fan.changed`
  - emitted when fan RPM telemetry changes
- `openrgb.changed`
  - emitted when OpenRGB status changes
- `config.changed`
  - emitted when the serialized daemon config fingerprint changes

### Noise reduction
Important filtering rule:
- fan RPM-only deltas no longer also trigger `device.updated`

That keeps the event stream smaller and makes event semantics cleaner for frontend consumers.

## Load Reduction
The backend reduces daemon load by:
- using fast polling only for lightweight runtime state
- keeping `GetConfig` on a slower cadence
- forcing extra config reads only when reconnect or topology changes justify it
- emitting no events at all for identical snapshots

## Known Limits
- there is still no native daemon push stream
- config-change detection is based on a serialized config fingerprint
- telemetry is still polling-based, so update cadence is bounded by the poll interval

## Summary
- fallback polling is now explicit, change-aware, and rate-limited
- the backend forwards only relevant deltas
- this is the correct bridge until daemon-side subscriptions exist
