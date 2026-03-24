# Domain Model

This document defines the current web-facing domain model for the Lian Li Linux web controller.
It translates daemon-backed data into the concepts that matter to frontend routes, backend API documents, and user-facing workflows.

- Status: implemented
- Audience: contributors, maintainers
- Source of truth: shared types, backend models, and current device API documents
- Last reviewed: 2026-03-14
- Related documents:
  - `system-overview.md`
  - `daemon-integration.md`
  - `../architecture/source-of-truth.md`

## Core Entities

### Device

A device is a controllable hardware unit exposed through the backend device inventory.
Important current properties include:

- stable `id`
- human-readable `name`
- `family`
- `online` state
- capability summary
- current telemetry and stream-related state

### Device Capability

Capabilities describe what a device can currently do through the web surface.
Important current capability flags include:

- RGB support
- RGB zone count
- fan support
- fan slot count
- LCD support
- pump support
- motherboard-sync support
- per-fan control support

### Device State

The current device snapshot can include:

- fan RPM telemetry
- coolant temperature
- streaming activity
- other live hardware-derived values surfaced by the backend

This state is backend-derived and should be treated as readonly in the frontend.

### Lighting State

Lighting state is modeled per device and, when applicable, per zone.
Important current values include:

- zone number
- effect
- colors
- brightness percentage

The current MVP lighting UI operates on one selected device at a time.

### Fan State

Fan state is modeled per selected device and includes:

- slot-level mode
- configured manual percent when available
- RPM telemetry
- update interval

The current MVP fan UI applies one manual percentage across all reported fan slots of the selected device.

### Profile

A profile is a backend-stored reusable preset that can target:

- all devices
- a selected set of device IDs

The current MVP profile model can store:

- metadata such as id, name, and description
- lighting settings
- manual fan settings

Profiles are intentionally independent from raw daemon config snapshots.

## Identity and Addressing

Device IDs come from daemon and shared-type logic.
Current common forms include:

- `hid:<serial>` or `hid:<vid>:<pid>:<bus-portpath>` for wired devices
- `wireless:<mac>` for wireless targets
- child IDs such as `<base_id>:port<index>` for multi-port controllers

The web layer should treat these IDs as stable integration identifiers, not user-facing names.

## Current Modeling Rules

- Capability truth comes from daemon-backed device modeling, not from frontend guesses.
- User-facing names may differ from technical IDs, but the backend device ID remains the authoritative identifier.
- Frontend workbench forms may hold draft values, but the persisted or live state remains backend-confirmed.
- Profiles represent user intent and orchestration, not raw low-level config ownership.

## Current Gaps

- Inventory ordering, renaming, controller grouping, and richer wireless metadata belong to later phases.
- Capability matrices for lighting, fans, wireless, and LCD/media are not yet split into their future dedicated documents.
- The current domain model is sufficient for MVP routes, but later workbenches will extend it with richer targeting and compatibility metadata.
