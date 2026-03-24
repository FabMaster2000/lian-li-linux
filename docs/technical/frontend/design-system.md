# Design System

This document records the design-system foundation introduced in Phase 14.
It defines the token sources, reusable primitives, page anatomy, and responsive rules that now govern the frontend shell.

- Status: implemented
- Audience: contributors, maintainers
- Source of truth: Phase 14 frontend token and component implementation
- Last reviewed: 2026-03-14
- Related documents:
  - `current-information-architecture.md`
  - `target-information-architecture.md`
  - `../development/documentation-style-guide.md`

## Token Sources

The current token and theme sources are:

- `frontend/src/styles/tokens.css`
- `frontend/src/styles/theme.css`
- `frontend/src/styles/global.css`

## Token Categories

### Spacing Scale

The system uses a named spacing scale:

- `--space-1` to `--space-8`

These tokens drive shell spacing, card padding, gaps, and responsive compaction.

### Typography Scale

The system uses:

- `--font-display` for page titles, section titles, and emphasis
- `--font-body` for operational copy
- `--text-xs` to `--text-2xl` for scaled UI text roles

### Semantic Color Roles

The theme defines:

- canvas and surface backgrounds
- soft and strong border colors
- primary text and muted text
- warm accent, teal accent, and ice accent

### Status Colors

The current semantic status colors are:

- success
- warning
- danger
- info

These drive badges, notices, and action-state emphasis.

### Surface Layering

The current shell uses:

- a dark canvas gradient
- glass-like elevated panels
- strong cards for primary work areas
- lighter soft surfaces for embedded controls

### Form Size Rules

Current form controls use:

- minimum interactive height around 44 to 46 pixels
- pill or rounded-medium corners
- explicit readonly and disabled states
- consistent label placement above controls

### Responsive Breakpoints

The current breakpoint model is:

- large: `72rem`
- medium: `58rem`
- small: `40rem`

The shell collapses from a desktop sidebar layout into a stacked layout while preserving the same page order.

## Reusable Components

### Layout

- `AppShell`
- `SidebarNav`
- `TopBar`
- `PageHeader`
- `SectionHeader`

### UI

- `Card`
- `Panel`
- `Tabs`
- `StatusBadge`
- `StatTile`

### Devices

- `DeviceCard`
- `InventoryDeviceCard`

### Forms

- `ColorField`
- `EffectSelect`
- `SliderField`

### Feedback

- `ActionBar`
- `ConfirmDialog`
- `InlineNotice`
- `EmptyState`
- `ErrorState`
- `LoadingSkeleton`

## Page Anatomy Rules

Each route should now be built from:

- `PageHeader` for title, description, and route-level actions
- `StatTile` summary strip for context
- `Panel`, `Card`, or route-specific workbench cards for the main content
- `InlineNotice` for explicit state communication
- `ActionBar` when the page has a focused apply or save step

## Motion

The current shell uses lightweight purposeful motion:

- page content rises in on first render
- skeleton lines shimmer during loading
- navigation cards and buttons lift slightly on hover

Motion is meant to reinforce hierarchy and state transitions, not to distract from operator tasks.

## Implementation Notes

- All component imports point directly to the organized subdirectories (`components/ui/`, `components/layout/`, `components/feedback/`, `components/forms/`, `components/devices/`). Legacy root-level re-export shims have been removed.
- The design-system foundation is intentionally incremental. Later phases can deepen the components without replacing the shell pattern again.
- All active pages follow the same component system so future workbenches inherit a stable visual grammar from the start.
