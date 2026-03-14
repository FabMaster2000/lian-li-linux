# Profile Storage

## Scope
This document records the storage decision for Phase 6.5.
It covers where profiles are stored, why that storage was chosen, and which guarantees the
backend currently provides.

## Decision
The MVP uses a JSON file store.

Chosen path:
- default: `~/.config/lianli/profiles.json`
- derived from the daemon config directory, next to `config.json`

Current implementation:
- path resolution in `backend/src/config/mod.rs`
- storage layer in `backend/src/storage/mod.rs`

## Why JSON
JSON was chosen over SQLite for the MVP because it is:
- already available in the current Rust backend without extra services
- easy to inspect and repair manually on a headless Ubuntu VM
- sufficient for the current profile volume and access pattern
- simple to back up alongside the daemon config directory

SQLite remains a valid later option if profile history, richer queries, imports/exports, or
concurrent multi-process writes become real requirements.

## File Format
The backend stores a versioned document:

```json
{
  "version": 1,
  "profiles": [
    {
      "id": "night-mode",
      "name": "Night Mode",
      "description": "Dim lighting and reduce fan speed",
      "targets": { "mode": "all", "device_ids": [] },
      "lighting": {
        "enabled": true,
        "color": "#223366",
        "effect": "Static",
        "brightness_percent": 15
      },
      "fans": {
        "enabled": true,
        "mode": "manual",
        "percent": 25
      },
      "metadata": {
        "created_at": "2026-03-13T12:00:00Z",
        "updated_at": "2026-03-13T12:00:00Z"
      }
    }
  ]
}
```

Notes:
- `version` reserves room for later migrations
- `profiles` uses the web-level profile model, not raw daemon config

## Storage Guarantees
Current guarantees:
- missing store file is treated as an empty profile list
- parent directories are created automatically
- writes are serialized inside the backend process with an in-memory mutex
- writes use a temporary file plus rename step
- profile ordering is normalized by `name`, then `id`

What is not guaranteed:
- cross-process locking between multiple backend instances
- rollback for daemon-side runtime failures after profile apply
- migration support beyond the current `version = 1`

## Operational Notes
- `GET /api/runtime` exposes the resolved `profile_store_path`
- profile CRUD touches only `profiles.json`
- profile apply reads `profiles.json` and then writes the daemon `config.json` through IPC

## Test Coverage
Covered today:
- profile CRUD persistence through backend API tests
- storage behavior when the file is initially missing
- create/update/delete roundtrip in storage unit tests
- profile apply integration with a mocked daemon

## Summary
- JSON is the right MVP storage because it is simple, inspectable, and Ubuntu-friendly
- the storage layer is now isolated behind `ProfileStore`
- moving to SQLite later is possible without changing the profile API surface
