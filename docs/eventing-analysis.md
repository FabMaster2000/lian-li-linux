# Eventing Analysis

## Scope
This document analyzes the current daemon-side eventing situation for Phase 7.1.
The goal is to determine whether the existing codebase already provides push-based status updates,
or whether the backend must implement polling as the first live-update mechanism.

## Short Answer
- IPC event types exist in `lianli-shared`.
- A `Subscribe` request exists in the IPC protocol.
- The daemon does not implement event streaming today.
- The existing GUI uses polling, not subscriptions.
- For the current backend, polling is required.

## Existing Event Surface

### Shared IPC types
`crates/lianli-shared/src/ipc.rs` defines:
- `IpcRequest::Subscribe`
- `IpcEvent::DeviceAttached`
- `IpcEvent::DeviceDetached`
- `IpcEvent::ConfigChanged`
- `IpcEvent::FanSpeedUpdate`

This shows that the protocol was designed with future push events in mind.

### Current daemon implementation
`crates/lianli-daemon/src/ipc_server.rs` currently handles:
- `Ping`
- request/response RPCs like `ListDevices`, `GetConfig`, `GetTelemetry`
- config write requests such as `SetConfig`, `SetFanConfig`, `SetRgbConfig`, `SetLcdMedia`

For `Subscribe`, the daemon returns:
- `"Subscribe not yet implemented; use polling via GetTelemetry"`

That is the decisive implementation detail: the public IPC server does not provide a live stream.

## Transport Reality
The IPC transport is newline-delimited JSON over a Unix socket.
Technically, the transport could carry a stream of messages on one long-lived connection.

But the current daemon behavior is still request/response:
- client sends one request line
- daemon sends one response line
- no subscriber registry exists
- no `IpcEvent` lines are emitted to clients after the response

So the transport is stream-capable in principle, but not used as an event stream in practice.

## Internal Change Signals Inside the Daemon
The daemon does maintain internal mutable state that changes over time.
Important examples:

### Config reload flag
`crates/lianli-daemon/src/ipc_server.rs` sets `config_reload_pending = true` after:
- `SetConfig`
- `SetFanConfig`
- `SetRgbConfig`
- `SetLcdMedia`

`crates/lianli-daemon/src/service.rs` checks that flag in the main loop and reloads config.

This is an internal coordination mechanism only.
It is not exposed as a client-visible event.

### Periodic telemetry refresh
`crates/lianli-daemon/src/service.rs` updates IPC-visible shared state through:
- `sync_ipc_state()`
- `sync_ipc_telemetry()`

That shared state includes:
- current device list
- fan RPM telemetry
- coolant temperatures
- `streaming_active`
- OpenRGB server status

Again, these are not pushed to clients.
Clients only see them on the next poll.

## Existing Client Behavior
The current GUI confirms the intended runtime model:

`crates/lianli-gui/src/backend.rs` uses:
- polling every 2 seconds
- `Ping`
- `ListDevices`
- `GetTelemetry`

On reconnect or when device count changes, the GUI additionally reloads:
- `GetConfig`
- `GetRgbCapabilities`
- `ListDevices`

The GUI does not call `Subscribe`.
This is strong evidence that polling is the real supported mechanism today.

## What Can Be Observed Via Polling

### Available now
By polling existing daemon requests, the backend can detect:
- daemon reachable / unreachable
- device attached / detached
- fan RPM changes
- coolant temperature changes
- `streaming_active` changes
- OpenRGB server running/error changes

Using `GetConfig`, the backend can also detect persisted desired-state changes for:
- lighting config
- fan config
- LCD config

### Not available as native push
The daemon does not currently push:
- device hotplug notifications
- telemetry deltas
- config changed notifications
- RGB change notifications
- fan change notifications

## Recommendation For Phase 7

### MVP recommendation
Build the backend event abstraction on polling first.

Reason:
- it matches current daemon behavior
- it matches the existing GUI strategy
- it avoids modifying daemon IPC before the web stack is proven
- it can be implemented immediately using `ListDevices`, `GetTelemetry`, and optional `GetConfig`

### Suggested polling sources
- fast poll:
  - `Ping`
  - `ListDevices`
  - `GetTelemetry`
- slower or change-triggered poll:
  - `GetConfig`

Recommended initial strategy:
- poll daemon reachability and telemetry every 1-2 seconds
- diff device list and telemetry snapshots in the backend
- emit normalized web events only when something actually changed
- refresh full config after successful backend write operations or when device topology changes

### Suggested normalized backend events
- `daemon.connected`
- `daemon.disconnected`
- `device.added`
- `device.removed`
- `device.updated`
- `fan.changed`
- `lighting.changed`
- `config.changed`
- `openrgb.changed`

## Future Native Eventing Option
If later needed, `Subscribe` can be implemented in the daemon.
That would require at least:
- long-lived IPC connections
- a subscriber registry
- serialization of `IpcEvent` values to subscribed clients
- lifecycle handling for disconnects
- emission points from device discovery, config writes, and telemetry updates

This is feasible, but it is new daemon work, not a currently available capability.

## Conclusion
- The codebase defines event concepts, but does not currently ship daemon-to-client push events.
- The real operational model is polling.
- Backend live updates in the next steps should therefore start with polling plus diffing.
- A native daemon `Subscribe` stream can remain a later enhancement.

Follow-up:
- the implemented backend event envelope for this polling-based approach is documented in
  `docs/event-model.md`
- the concrete fallback polling cadence and filtering rules are documented in
  `docs/polling-fallback.md`
