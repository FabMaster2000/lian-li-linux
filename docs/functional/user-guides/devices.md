# Devices

This guide explains how to use the current Devices overview and the richer Device Detail workspace.

- Status: implemented
- Audience: users, operators, testers
- Source of truth: current Devices and Device Detail routes
- Last reviewed: 2026-03-15
- Related documents:
  - `dashboard.md`
  - `lighting.md`
  - `fans.md`
  - `../feature-specs/device-dashboard.md`

## Devices Overview

The Devices route is now a full inventory page instead of a simple selection screen.

It shows:

- device count and controller count summaries
- controller summary cards
- search
- filter by family
- filter by capability or attention state
- grouping by controller or family
- quick actions on each device card

Each device card includes:

- display name
- controller label
- physical role
- wireless context when relevant
- capability summary
- current mode summary
- health summary

## Quick Actions

From the overview you can:

- open the device detail route
- open Lighting for the selected device
- open Fans for the selected device
- open Diagnostics with the selected device context
- set a static color for RGB-capable devices
- apply a matching profile
- rename and reorder the device presentation

## Rename and Ordering

The `Rename / order` dialog persists these values in backend storage:

- display name
- stable UI order
- physical role
- controller label
- cluster or group label

These settings survive reloads because they are stored by the backend rather than kept only in the browser.

## Device Detail Page

The detail page is now a device workspace.
It includes:

- identity and presentation metadata
- controller and wireless context
- capability breakdown
- diagnostics summary
- current lighting summary
- current fan summary
- assigned profiles
- links to Lighting, Fans, and Diagnostics for the selected device

## Typical Workflow

1. Open Devices.
2. Search, filter, or regroup until the target device or controller is visible.
3. Use quick actions for small inventory-level tasks.
4. Open Details for deeper review.
5. Continue into Lighting or Fans with the selected device already carried into that route.

## Current Limitations

- Controller grouping is derived from the current device ID model rather than from a full explicit topology graph.
- Diagnostics focus is now carried into the route, but the richer diagnostics workbench arrives in a later phase.
- Wireless topology remains intentionally pragmatic for the current discovery model.
