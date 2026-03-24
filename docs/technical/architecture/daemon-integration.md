# Daemon Integration

This document describes how the web stack integrates with `lianli-daemon`.
It defines the current daemon boundary, the responsibilities that remain daemon-owned, and the rules the web backend must follow when orchestrating hardware-facing actions.

- Status: implemented
- Audience: contributors, maintainers, operators
- Source of truth: daemon IPC behavior and backend daemon client integration
- Last reviewed: 2026-03-14
- Related documents:
  - `system-overview.md`
  - `backend-architecture.md`
  - `../architecture/source-of-truth.md`

## Integration Boundary

The daemon remains the only supported owner of:

- hardware discovery
- transport and controller communication
- device-family-specific low-level behavior
- daemon config loading and application
- direct interaction with device capabilities such as RGB, fan, and LCD settings

The web backend may orchestrate and reformat daemon-backed behavior, but it must not bypass the daemon to talk to hardware directly.

## Current Integration Path

```text
frontend action
  -> backend handler
  -> daemon client
  -> Unix socket IPC request
  -> daemon service logic
  -> device / transport layer
```

## Current IPC-Backed Responsibilities

The backend currently uses daemon-backed operations for:

- daemon health and reachability checks
- device inventory and device detail reads
- telemetry snapshots
- config reads and writes
- lighting state reads and RGB config writes
- fan state reads and manual fan config writes
- profile application through config orchestration

## Configuration Model

Two important configuration categories exist:

- daemon-owned application config, which drives device behavior
- backend-owned runtime config, which controls HTTP serving, auth, paths, and profile storage

The backend may translate browser input into daemon config documents, but daemon config semantics remain daemon-owned.

## Event and Refresh Behavior

Native daemon push is not yet the only live-update mechanism.
The current web stack therefore uses:

- backend-side event publication after successful write operations
- polling-based refresh for live-ish synchronization when needed

This means live updates are useful today, but still bounded by current daemon and backend event support.

## Integration Rules

- Reuse existing IPC types from `lianli-shared` before inventing new web-only transport models.
- Do not duplicate device capability logic that already lives in daemon or shared types.
- Keep backend request validation explicit, but let the daemon remain authoritative for low-level feasibility.
- Document unsupported or partial operations rather than silently simulating full support in the web layer.

## Operational Notes

- The backend expects a reachable daemon socket.
- The dashboard and runtime endpoints already expose daemon path and reachability information for diagnostics.
- When the daemon is offline, frontend pages should show degraded or disabled control states instead of pretending writes succeeded.

## Current Gaps

- Some richer workbench behaviors planned for later phases still depend on deeper daemon support or clearer capability matrices.
- Wireless, LCD/media, and advanced fan workflows are not yet documented as fully separate integration surfaces.
- Eventing documentation is still in migration from legacy flat docs into the new target structure.
