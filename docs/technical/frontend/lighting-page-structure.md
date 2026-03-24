# Lighting Page Structure

This document describes the current page anatomy of the Phase 16 Lighting Workbench.
It is meant to keep the UI structure stable while later phases deepen device-family capability handling.

- Status: implemented
- Audience: contributors, frontend maintainers
- Source of truth: `frontend/src/pages/LightingPage.tsx` and `frontend/src/hooks/useLightingWorkbenchData.ts`
- Last reviewed: 2026-03-15
- Related documents:
  - `current-information-architecture.md`
  - `target-information-architecture.md`
  - `../architecture/lighting-workbench-model.md`

## Page Anatomy

The page now follows this high-level structure:

1. page intro with runtime context and quick navigation
2. summary strip with primary device, target mode, target count, and pending state
3. target selection section
4. presets section
5. effect editor section
6. preview and capability guidance section
7. pending changes action bar
8. live-state and apply-report stack

## Section Responsibilities

### Page Intro

The intro establishes:

- primary editing device
- current target mode
- current live-state activity
- refresh action
- device-detail deep link

### Summary Strip

The summary strip gives the user immediate answers to:

- which device is acting as the editing baseline
- how broadly the draft will apply
- how many targets are involved
- whether the draft is dirty

### Target Selection Section

This section contains:

- target-mode tabs for `single`, `selected`, and `all`
- the primary device selector
- zone selector
- zone apply mode selector
- optional selected-device checklist
- `sync selected devices` toggle for selected-target workflows

The primary device remains separate from the target scope by design.

### Presets Section

This section contains:

- built-in presets
- preset-name field
- `Save as preset`
- browser-local custom preset list

This keeps curated and custom entry points visible without burying them in dialogs.

### Effect Editor Section

The effect editor contains the draft controls:

- effect selector
- palette editing
- speed
- brightness
- direction
- scope

Effect-specific panels are handled by visibility rules rather than entirely separate routes.
For example, palette controls disappear for non-palette effects.

### Preview and Guidance Section

This section intentionally mixes:

- a pending draft summary
- the current live zone summary
- target summary text
- capability warnings
- effect notes

It gives the user one place to sanity-check the draft before applying.

### Pending Changes Action Bar

The action bar is the explicit commit surface.
It contains:

- Apply
- Reset / Revert
- Apply to all compatible

Its summary text changes depending on whether the draft is dirty.

### Live State and Apply Report Stack

The lower stack contains:

- current readonly zone state for the primary device
- optional zone-switch cards when multiple zones exist
- optional last-apply report for partial success review

## Design Notes

The page intentionally avoids a wizard or modal-heavy flow.
Lighting is frequent, iterative work, so the structure keeps targets, draft, preview, and apply actions visible at the same time.

## Current Constraints

- the primary device is the only live preview source
- target groups are apply-time concepts, not permanent sync groups
- custom presets are browser-local
- family-specific effect constraints are warnings, not a complete backend capability graph
