# Dashboard

This guide explains what the current dashboard shows and how to use it as the entry point into the web controller.

- Status: implemented
- Audience: users, operators, testers
- Source of truth: current dashboard route and backend-backed dashboard data
- Last reviewed: 2026-03-14
- Related documents:
  - `getting-started.md`
  - `devices.md`
  - `../../technical/architecture/frontend-architecture.md`

## What The Dashboard Shows

The current dashboard combines:

- device counts
- RGB-capable and fan-capable totals
- daemon reachability
- runtime paths for backend and daemon integration
- live device inventory cards
- quick entry points into the main control areas

## Typical Workflow

1. Open the dashboard after loading the web UI.
2. Check whether the daemon is reachable.
3. Review the current device inventory and capability counts.
4. Use the device cards or section entry points to continue into Devices, Lighting, Fans, or Profiles.
5. Use the refresh action if you need a new backend snapshot.

## What To Watch

- `Daemon`: whether backend-to-daemon communication is currently working
- `Socket`: which daemon socket path the backend is using
- `Updated`: whether the current snapshot is loading, refreshing, or already synced
- runtime panels: backend host, backend port, profile-store path, daemon config path, and XDG paths

## Current Limitations

- The dashboard is still a foundation route rather than a full fleet operations workspace.
- It does not yet provide advanced filtering, grouping, orchestration, or detailed diagnostics history.
- Some navigation cards still describe future route expansion rather than a fully mature information architecture.
