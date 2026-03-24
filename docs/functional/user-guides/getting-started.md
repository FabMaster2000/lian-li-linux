# Getting Started

This guide explains how to reach the current web controller, verify that the stack is connected, and open the first control surfaces.

- Status: partial
- Audience: users, operators, testers
- Source of truth: current frontend routes and backend runtime behavior
- Last reviewed: 2026-03-14
- Related documents:
  - `dashboard.md`
  - `devices.md`
  - `../../technical/architecture/system-overview.md`

## What You Need

- a running `lianli-daemon`
- a running `lianli-backend`
- the frontend served in a browser
- at least one simulated or physical device if you want to test device-specific workflows

## First Checks

1. Open the web UI in your browser.
2. Confirm that the dashboard loads instead of showing a fatal load error.
3. Check the daemon status and runtime path summary on the dashboard.
4. Confirm that the device inventory section shows one or more devices, or explicitly reports that none are available.

## Main Areas

The current MVP-plus web UI exposes these main routes:

- Dashboard
- Devices
- Lighting
- Fans
- Profiles
- Settings

The most common starting flow is:

1. open Dashboard
2. review daemon status and current inventory
3. open Devices and choose a controller
4. continue into Lighting, Fans, or Profiles as needed

## Expected Current Behavior

- Dashboard shows system and inventory context
- Devices requires explicit device selection before opening detail
- Lighting controls one RGB-capable device at a time
- Fans controls one fan-capable device at a time with a fixed manual percentage
- Profiles stores reusable lighting and fan presets in the backend
- Settings is currently a placeholder for future diagnostics and runtime controls

## Current Limitations

- The application is still in documentation and UX migration, so some routes are broader placeholders for later phases.
- Wireless, LCD/media, and richer diagnostics workbenches are not available as first-class routes yet.
- Fan curves, batch orchestration, import/export, and advanced presets belong to later phases.
