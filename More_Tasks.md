# Post-MVP Execution Specification
## Lian Li Linux Web Controller
## UX Redesign, Feature Expansion, and Documentation Overhaul
## Language Rule: English Only

---

# 1. Mission

Continue the existing Linux-native Lian Li web controller beyond the MVP.

The next development cycle must focus on four major outcomes:

1. **Make the UI cleaner, more structured, and more scalable**
2. **Add feature depth inspired by current L-Connect 3 workflows**
3. **Strengthen documentation quality and structure**
4. **Preserve the existing daemon-backed Linux architecture**

This document is written for a **Coding AI** and should be used as an implementation contract.

---

# 2. Core Product Goals

The system should evolve from a technically working MVP into a web-based control suite that offers:

- clearer navigation
- stronger device overview and grouping
- better lighting workflows
- better fan workflows
- wireless-specific management surfaces
- LCD/media workflows where supported
- stronger profile and preset handling
- diagnostics and settings pages
- better operator confidence through structure, state visibility, and documentation

The system should remain:

- Linux-native
- daemon-backed
- headless-compatible
- remote-accessible through the browser
- modular and well documented

---

# 3. Architecture Constraints

The Coding AI must preserve this architecture:

```text
Browser UI
  -> Frontend
    -> Backend API
      -> Daemon Adapter / IPC Layer
        -> lianli-daemon
          -> transport / device layer
            -> controller / hardware
```

## Non-negotiable rules

- Do **not** move hardware communication into the frontend
- Do **not** bypass the daemon unless absolutely necessary and explicitly documented
- Do **not** duplicate transport logic already implemented in the daemon stack
- Reuse existing backend and daemon capabilities before introducing new abstractions
- Document all feature gaps explicitly

---

# 4. Documentation Rule

All documentation must be written in **English only**.

This includes:

- README files
- architecture documents
- developer docs
- user guides
- feature specs
- admin guides
- troubleshooting docs
- changelogs
- release notes
- test plans
- ADRs
- comments in high-level documentation

---

# 5. Documentation Target Structure

All documentation must be reorganized into two top-level categories:

```text
docs/
  technical/
  functional/
```

---

## 5.1 Technical Documentation

Technical documentation is for:

- developers
- maintainers
- contributors
- operators
- infrastructure owners

### Target structure

```text
docs/technical/
  README.md

  architecture/
    system-overview.md
    source-of-truth.md
    domain-model.md
    daemon-integration.md
    ipc-mapping.md
    eventing.md
    device-ordering-and-labeling.md
    lighting-capability-matrix.md
    lighting-workbench-model.md
    fan-capability-matrix.md
    fan-workbench-model.md
    wireless-capability-matrix.md
    wireless-workbench-model.md
    lcd-capability-matrix.md
    lcd-workbench-model.md
    profile-model-v2.md
    access-model.md

  frontend/
    current-information-architecture.md
    target-information-architecture.md
    design-system.md
    component-inventory.md
    state-management.md
    route-map.md
    lighting-page-structure.md

  backend/
    api-overview.md
    endpoints.md
    websocket-events.md
    error-model.md
    orchestration-model.md
    profile-storage.md
    import-export-model.md
    fan-curve-api.md
    wireless-api.md
    lcd-media-api.md
    observability.md

  deployment/
    ubuntu-headless-setup.md
    configuration.md
    systemd.md
    reverse-proxy.md
    security.md
    observability.md
    backups.md

  testing/
    backend-tests.md
    frontend-tests.md
    integration-tests.md
    manual-test-plan.md
    regression-checklist.md

  development/
    local-dev-setup.md
    code-structure.md
    coding-guidelines.md
    documentation-style-guide.md
    release-process.md
    doc-audit.md

  adr/
    adr-001-documentation-structure.md
    adr-002-information-architecture.md
    adr-003-design-system.md
    adr-004-device-model.md
    adr-005-lighting-workbench.md
    adr-006-fan-workbench.md
    adr-007-profile-model.md
    adr-008-lcd-media-workbench.md
    adr-009-auth-and-roles.md
```

---

## 5.2 Functional Documentation

Functional documentation is for:

- end users
- homelab operators
- admins
- testers
- reviewers

### Target structure

```text
docs/functional/
  README.md

  user-guides/
    getting-started.md
    dashboard.md
    devices.md
    lighting.md
    fans.md
    fan-curves.md
    wireless-sync.md
    profiles.md
    lcd-media.md
    settings.md
    diagnostics.md

  admin-guides/
    authentication.md
    deployment-basics.md
    backups-and-restore.md
    user-access.md

  feature-specs/
    ui-foundation.md
    device-dashboard.md
    lighting-workbench.md
    fan-workbench.md
    wireless-workbench.md
    lcd-workbench.md
    profile-system.md
    diagnostics-surface.md
    settings-and-update-surface.md
    import-export-workflow.md

  troubleshooting/
    daemon-unreachable.md
    backend-unreachable.md
    device-not-found.md
    lighting-sync-issues.md
    fan-control-issues.md
    wireless-binding-issues.md
    import-export-issues.md
    lcd-media-issues.md

  glossary/
    terms.md

  release-notes/
    roadmap.md
    changelog.md
```

---

# 6. Global Working Rules for the Coding AI

For every feature or phase:

## Before implementation
- inspect the daemon, backend, and frontend state
- document what already exists
- document what can be reused
- document what is missing
- identify whether the work is:
  - frontend-only
  - backend-only
  - daemon-extension
  - documentation-only
  - multi-layer

## During implementation
- prefer incremental changes
- keep modules cleanly separated
- avoid large unrelated refactors
- make UI behavior explicit
- make errors visible
- preserve backward compatibility where reasonable

## After implementation
- update both technical and functional docs
- add or update tests
- document known limitations
- recommend the next logical phase

---

# 7. Required End-of-Phase Report Format

At the end of **every phase**, the Coding AI must deliver the following sections:

## A. Summary
- what was implemented
- what changed conceptually

## B. Files changed
- modified files
- new files

## C. Architectural decisions
- what was decided
- why it was chosen
- alternatives considered or rejected

## D. Tests
- what was tested
- how to reproduce
- expected results

## E. Known limitations
- what is still missing
- what depends on daemon work
- what was intentionally deferred

## F. Documentation
- technical docs added or updated
- functional docs added or updated

## G. Next recommended phase
- what should be done next
- blockers or risks if any

---

# 8. Meta-Tasks Before the New Feature Phases

---

## Meta-Task A — Full Documentation Audit

### Goal
Review all existing documentation and classify each file.

### Required output
Create:

```text
docs/technical/development/doc-audit.md
```

### Required table
| Current File | Current Purpose | Current Audience | Target Location | Action | Reason |
|---|---|---|---|---|---|

### Allowed actions
- keep
- move
- rewrite
- split
- merge
- deprecate
- delete

---

## Meta-Task B — Documentation Style Guide

### Goal
Create one documentation standard for the whole project.

### Required output
Create:

```text
docs/technical/development/documentation-style-guide.md
```

### Must include
- English-only rule
- heading conventions
- glossary rules
- terminology rules
- code block conventions
- API example conventions
- diagram rules
- screenshot policy
- versioning and review notes
- deprecation notes
- feature-status labels such as:
  - planned
  - partial
  - implemented
  - deprecated

---

## Meta-Task C — Source-of-Truth Matrix

### Goal
Define where the authoritative truth lives for each concern.

### Required output
Create:

```text
docs/technical/architecture/source-of-truth.md
```

### Example structure
| Concern | Source of Truth |
|---|---|
| Device transport behavior | daemon / transport layer |
| API contract | backend API docs |
| Route structure | frontend architecture docs |
| User workflows | functional docs |
| Deployment steps | technical deployment docs |
| Profile semantics | technical profile model + functional feature spec |

---

## Meta-Task D — Documentation Indexes

### Goal
Make the docs easy to navigate.

### Required files
- `docs/README.md`
- `docs/technical/README.md`
- `docs/functional/README.md`

### Each index must explain
- target audience
- what belongs there
- how to navigate
- how to contribute

---

# 9. Phase Order

The Coding AI must implement the following phases in this order unless a blocker requires a justified change:

1. Phase 13 — Documentation Overhaul Foundation  
2. Phase 14 — Information Architecture and Visual System Redesign  
3. Phase 15 — Device Dashboard, Inventory, and Topology Experience  
4. Phase 16 — Lighting Workbench 2.0  
5. Phase 17 — Fan Workbench 2.0  
6. Phase 18 — Wireless Sync and Binding Workbench  
7. Phase 19 — LCD / Media Workbench  
8. Phase 20 — Profiles 2.0, Import/Export, and Preset Architecture  
9. Phase 21 — Diagnostics, System Info, Settings, and Update Surface  
10. Phase 22 — Multi-Device Orchestration and Batch Operations  
11. Phase 23 — Access Model, Roles, Permissions, and Auditability  
12. Phase 24 — Final Documentation Hardening and Productization Pass

---

# 10. Phase 13 — Documentation Overhaul Foundation

## Goal
Create the new documentation baseline and clean separation between technical and functional documentation.

## Why this phase exists
The project has moved beyond MVP.  
Documentation must now support both developers and users without mixing concerns.

## Required analysis
- audit all current documentation
- identify all non-English files
- identify duplicated, stale, or misplaced docs
- identify missing foundational docs

## Implementation tasks
- create the full documentation tree
- move or rewrite all existing docs into the new structure
- rewrite non-English docs into English
- add front matter or intro metadata to major docs
- create foundational architecture docs
- create foundational user-facing docs
- ensure the docs are navigable through index pages

## Required files
At minimum create or rewrite:

### Technical
- `docs/technical/README.md`
- `docs/technical/architecture/system-overview.md`
- `docs/technical/architecture/backend-architecture.md`
- `docs/technical/architecture/frontend-architecture.md`
- `docs/technical/architecture/daemon-integration.md`
- `docs/technical/architecture/domain-model.md`

### Functional
- `docs/functional/README.md`
- `docs/functional/user-guides/getting-started.md`
- `docs/functional/user-guides/dashboard.md`
- `docs/functional/user-guides/devices.md`
- `docs/functional/user-guides/lighting.md`
- `docs/functional/user-guides/fans.md`
- `docs/functional/user-guides/profiles.md`

## Acceptance criteria
- all docs are classified
- all docs are English-only
- technical and functional docs are clearly separated
- central indexes exist and work

---

# 11. Phase 14 — Information Architecture and Visual System Redesign

## Goal
Redesign the frontend structure so the application feels like one coherent control suite.

## Why this phase exists
A broader feature set needs stronger structure.  
Without this phase, later pages will create visual and navigational clutter.

## Required analysis
- audit current routes and pages
- identify duplicated UI patterns
- identify inconsistent naming and layout behavior
- identify current UX pain points

## Implementation tasks

### 14.1 Current information architecture audit
Create:

```text
docs/technical/frontend/current-information-architecture.md
```

Document:
- current route map
- current page responsibilities
- current loading patterns
- duplicated or inconsistent UX elements

### 14.2 Target information architecture
Create:

```text
docs/technical/frontend/target-information-architecture.md
```

Define the target top-level navigation:
- Dashboard
- Devices
- Lighting
- Fans
- Wireless Sync
- LCD / Media
- Profiles
- Diagnostics
- Settings
- Help / Docs

### 14.3 Design system foundation
Create reusable components:
- AppShell
- SidebarNav
- TopBar
- PageHeader
- SectionHeader
- Card
- Panel
- Tabs
- StatusBadge
- DeviceCard
- StatTile
- ActionBar
- ConfirmDialog
- InlineNotice
- EmptyState
- ErrorState
- LoadingSkeleton

Suggested structure:

```text
frontend/src/components/ui/
frontend/src/components/layout/
frontend/src/components/devices/
frontend/src/components/forms/
frontend/src/components/feedback/
```

### 14.4 Visual tokens and theme roles
Define:
- spacing scale
- typography scale
- semantic color roles
- status colors
- surface layering
- form size rules
- responsive breakpoints

Create:
- theme/token source files
- `docs/technical/frontend/design-system.md`
- `docs/functional/feature-specs/ui-foundation.md`

### 14.5 Standardize page anatomy
Every page should follow:
1. page header
2. context summary strip
3. primary work area
4. secondary information area
5. pending action bar if needed

### 14.6 Improve responsive behavior
Make the UI robust on tablets and smaller screens without breaking desktop-first layout.

## Acceptance criteria
- coherent app shell exists
- navigation is consistent
- visual primitives are reusable
- pages follow one structural pattern

---

# 12. Phase 15 — Device Dashboard, Inventory, and Topology Experience

## Goal
Build a richer device overview that helps users understand what is connected and how devices relate to controllers, groups, and capabilities.

## Why this phase exists
A better inventory model improves usability across all later workbenches.

## Required analysis
- inspect current device model in backend and frontend
- identify which controller/group/channel metadata already exists
- identify which metadata must be stored in backend UI metadata instead of daemon state

## Implementation tasks

### 15.1 Extend the web-facing device model
Add fields where available:
- display name
- stable UI order
- controller association
- wireless group / channel metadata
- physical role
- capability summary
- current mode summary
- health state

### 15.2 Create a dedicated Devices Overview page
The page must support:
- search
- filter by type
- filter by capability
- status badges
- quick actions
- grouping by controller or family

### 15.3 Device detail workspaces
Each device page must show:
- identity
- controller / wireless context
- capabilities
- current lighting summary
- current fan summary
- assigned profiles
- diagnostics summary
- links to lighting and fan workbenches

### 15.4 Rename and reorder workflows
Add:
- rename device
- rename group / cluster if applicable
- persist UI ordering and labels in backend storage if daemon support is missing

Create:
- `docs/technical/architecture/device-ordering-and-labeling.md`
- `docs/functional/feature-specs/device-dashboard.md`

### 15.5 Quick actions from inventory
Support:
- open lighting
- open fans
- set static color
- apply profile
- open diagnostics

### 15.6 Controller summary cards
Show:
- controllers detected
- associated devices
- group/channel counts
- controller state summary

## Acceptance criteria
- users can understand connected devices at a glance
- rename and ordering workflows exist
- device workspaces are easier to discover
- inventory model is clearly documented

---

# 13. Phase 16 — Lighting Workbench 2.0

## Goal
Build a richer lighting workspace inspired by sync/global apply/per-device lighting workflows.

## Why this phase exists
Lighting is one of the primary product surfaces and should become a structured workbench instead of a simple form.

## Required analysis
- inspect daemon and backend support for:
  - static color
  - brightness
  - effects
  - zone/group targeting
  - merged/synced effect behavior
  - unsupported operations by device type

## Implementation tasks

### 16.1 Capability matrix
Create:

```text
docs/technical/architecture/lighting-capability-matrix.md
```

Document:
- supported operations per device family
- unsupported operations
- backend limitations
- daemon limitations

### 16.2 Workbench model
Create:

```text
docs/technical/architecture/lighting-workbench-model.md
```

Model:
- target scope
- device selection
- zone/group selection
- color model
- effect model
- brightness model
- apply strategy
- preview state
- pending changes state
- partial success handling

### 16.3 Rebuild Lighting page
The page must include:
- target selector
- global / selected / single-device modes
- quick presets
- custom color picker
- brightness slider
- effect selector
- effect-specific option panels
- pending changes bar
- preview summary
- capability warnings

### 16.4 Multi-device lighting apply
Support:
- apply to selected devices
- apply to all compatible devices
- sync selected devices
- partial success reporting
- unsupported target reporting

### 16.5 Built-in lighting presets
Add curated presets such as:
- Static White
- Static Blue
- Static Red
- Rainbow
- Breathing
- Wave
- Warm Amber
- Performance Red

### 16.6 Advanced palette and effect parameters
Where supported, expose:
- palette editing
- speed
- direction
- layer count
- color count
- zone assignment

### 16.7 Improve save/apply workflow
Add:
- unsaved changes indication
- explicit Apply
- Reset / Revert
- Save as preset
- Apply to all compatible

### 16.8 Documentation
Create or update:
- `docs/technical/frontend/lighting-page-structure.md`
- `docs/functional/feature-specs/lighting-workbench.md`
- `docs/functional/user-guides/lighting.md`

## Acceptance criteria
- lighting page is a structured workbench
- multi-device apply exists
- presets exist
- unsupported cases are clearly handled
- docs are updated

---

# 14. Phase 17 — Fan Workbench 2.0

## Goal
Expand fan control from simple manual percentages into a structured fan-control workbench.

## Why this phase exists
Fan control is operationally sensitive and needs better UX, validation, and visibility.

## Required analysis
- inspect daemon support for:
  - manual fixed speed
  - fan curve CRUD
  - curve application
  - temperature source selection
  - start/stop mode
  - motherboard RPM sync
  - controller/group-level application
  - readback of current active mode

## Implementation tasks

### 17.1 Capability matrix
Create:

```text
docs/technical/architecture/fan-capability-matrix.md
```

### 17.2 Workbench model
Create:

```text
docs/technical/architecture/fan-workbench-model.md
```

Model:
- target scope
- manual mode
- curve mode
- start/stop mode
- motherboard sync
- temperature source
- curve points
- validation rules
- apply semantics
- warning state

### 17.3 Fan curve editor
Implement:
- list existing curves
- create curve
- rename curve
- duplicate curve
- delete curve
- edit curve points
- preview graph
- validation feedback
- apply curve to selected targets

### 17.4 Mode support
Support distinct modes:
- Manual fixed speed
- Curve mode
- Motherboard sync mode
- Start/Stop where supported

### 17.5 Batch fan operations
Support:
- apply to selected groups
- copy fan settings between compatible targets
- multi-target conflict reporting

### 17.6 Current-state telemetry surface
Display where possible:
- current RPM
- active mode
- temperature source
- active curve
- whether MB sync is enabled
- whether start/stop is enabled

### 17.7 Safe UX
Add:
- delete confirmations
- unsafe-setting warnings
- restore defaults action
- clear explanation when external sync disables software control

### 17.8 Documentation
Create or update:
- `docs/technical/backend/fan-curve-api.md`
- `docs/functional/feature-specs/fan-workbench.md`
- `docs/functional/user-guides/fans.md`
- `docs/functional/user-guides/fan-curves.md`

## Acceptance criteria
- users can manage fan curves in the browser
- manual, curve, sync, and start/stop modes are supported where possible
- UX is safe and structured
- docs are updated

---

# 15. Phase 18 — Wireless Sync and Binding Workbench

## Goal
Create a dedicated wireless-management experience.

## Why this phase exists
Wireless-capable devices need a UX that clearly explains controller relationships, grouping, and supported actions.

## Required analysis
- inspect backend and daemon support for:
  - controller visibility
  - wireless grouping
  - binding state
  - device identity state
  - channel assignment
  - identify behavior
  - rebind/unbind feasibility

## Implementation tasks

### 18.1 Capability matrix
Create:

```text
docs/technical/architecture/wireless-capability-matrix.md
```

### 18.2 Workbench model
Create:

```text
docs/technical/architecture/wireless-workbench-model.md
```

### 18.3 Wireless Sync page
The page should show:
- controllers
- bound devices
- available devices if exposed
- channel/group organization
- status hints
- device naming shortcuts
- identify actions if supported

### 18.4 Safe binding workflows
If supported:
- bind
- unbind
- rebind
- rename
- reorder

If not supported:
- expose read-only visibility and clearly document limitations

### 18.5 Conflict and compatibility messages
Examples:
- unsupported device type
- unavailable controller
- stale device state
- multi-controller conflict
- unknown state

### 18.6 Documentation
Create or update:
- `docs/technical/backend/wireless-api.md`
- `docs/functional/feature-specs/wireless-workbench.md`
- `docs/functional/user-guides/wireless-sync.md`
- `docs/functional/troubleshooting/wireless-binding-issues.md`

## Acceptance criteria
- wireless management has a dedicated page
- controller/device relationships are visible
- supported actions are exposed safely
- limitations are documented

---

# 16. Phase 19 — LCD / Media Workbench

## Goal
Expose supported LCD/media workflows in the browser.

## Why this phase exists
The daemon already supports LCD-related features, but the browser experience needs structure.

## Required analysis
- inspect support for:
  - image upload
  - GIF upload
  - video upload
  - conversion via ffmpeg
  - overlays
  - sensor descriptors
  - preview behavior
  - asset lifecycle

## Implementation tasks

### 19.1 Capability matrix
Create:

```text
docs/technical/architecture/lcd-capability-matrix.md
```

### 19.2 Workbench model
Create:

```text
docs/technical/architecture/lcd-workbench-model.md
```

### 19.3 LCD / Media page
The page should include:
- target picker
- asset library
- upload workflow
- preview area
- current assigned asset
- apply flow
- validation state

### 19.4 Upload and conversion workflow
Support:
- image upload
- GIF upload
- video upload
- validation messages
- conversion progress
- asset lifecycle states

### 19.5 Overlays and sensor widgets
Where supported, expose:
- text overlays
- time overlays
- sensor info overlays
- simple layout controls

### 19.6 Asset management
Support:
- list assets
- remove asset
- reuse asset
- assign asset to target
- duplicate asset metadata if useful

### 19.7 Documentation
Create or update:
- `docs/technical/backend/lcd-media-api.md`
- `docs/functional/feature-specs/lcd-workbench.md`
- `docs/functional/user-guides/lcd-media.md`
- `docs/functional/troubleshooting/lcd-media-issues.md`

## Acceptance criteria
- supported LCD workflows are browser-accessible
- upload and conversion are structured
- overlays are exposed where supported
- docs are updated

---

# 17. Phase 20 — Profiles 2.0, Import/Export, and Preset Architecture

## Goal
Make profiles richer, reusable, and closer to a real configuration system.

## Why this phase exists
Profiles are essential for repeatable operations, not just one-off adjustments.

## Required analysis
- inspect current profile model
- inspect how profiles are persisted
- inspect daemon/backend limitations for profile application
- identify how import/export should be versioned

## Implementation tasks

### 20.1 Profile model v2
Create:

```text
docs/technical/architecture/profile-model-v2.md
```

Extend profiles to support:
- lighting data
- manual fan settings
- fan curve references
- LCD/media references
- target scope
- tags
- description
- category
- import/export metadata
- compatibility hints
- conflict metadata

### 20.2 Richer profile management UI
Support:
- create
- edit
- duplicate
- delete
- apply
- preview summary
- tags
- category filtering
- compatibility state

### 20.3 Import/export workflows
Support:
- export profile
- import profile
- validate imported content
- show compatibility warnings
- include version metadata

### 20.4 Built-in system presets
Add built-in presets such as:
- Silent Cooling
- Balanced Cooling
- Performance Cooling
- Static White Showcase
- Rainbow Showcase
- LCD Diagnostics

### 20.5 Profile application reporting
Show:
- targets selected
- operations attempted
- skipped targets
- unsupported parts
- final result summary

### 20.6 Documentation
Create or update:
- `docs/technical/backend/import-export-model.md`
- `docs/functional/feature-specs/profile-system.md`
- `docs/functional/user-guides/profiles.md`
- `docs/functional/feature-specs/import-export-workflow.md`
- `docs/functional/troubleshooting/import-export-issues.md`

## Acceptance criteria
- profiles support more than basic lighting/manual fans
- import/export exists
- built-in presets are structured
- profile application is transparent

---

# 18. Phase 21 — Diagnostics, System Info, Settings, and Update Surface

## Goal
Create a polished operational surface for system status, diagnostics, settings, and update visibility.

## Why this phase exists
Operational clarity increases trust and makes the app feel like a full control suite rather than just a set of workbenches.

## Required analysis
- inspect current health/version/runtime endpoints
- inspect available backend telemetry
- inspect safe recovery actions
- inspect whether external update visibility should be passive or active

## Implementation tasks

### 21.1 Diagnostics page
The page should show:
- daemon connectivity
- backend health
- detected devices
- recent errors
- recent warnings
- WebSocket/event status
- config/storage status

### 21.2 Settings page
The page should consolidate:
- application settings
- UI preferences
- temperature unit
- backend/runtime config visibility
- auth hints
- integration toggles if available later

### 21.3 Update and version visibility
Show:
- backend version
- frontend version
- daemon version if known
- protocol/build metadata
- optional passive upstream version visibility if later desired

### 21.4 System information panels
Where available, show advisory system info such as:
- CPU temperature/load
- GPU temperature/load
- memory summary
- storage usage
- network summary

### 21.5 Support and recovery actions
Examples:
- reconnect attempt
- clear transient UI cache
- export diagnostics bundle
- view structured logs

### 21.6 Documentation
Create or update:
- `docs/technical/backend/observability.md`
- `docs/technical/deployment/configuration.md`
- `docs/functional/feature-specs/diagnostics-surface.md`
- `docs/functional/feature-specs/settings-and-update-surface.md`
- `docs/functional/user-guides/settings.md`
- `docs/functional/user-guides/diagnostics.md`

## Acceptance criteria
- diagnostics and settings have dedicated pages
- version/runtime visibility exists
- support/recovery actions are clearer
- docs are updated

---

# 19. Phase 22 — Multi-Device Orchestration and Batch Operations

## Goal
Make the application efficient for users who manage several compatible devices at once.

## Why this phase exists
Sync-style workflows need explicit orchestration and conflict visibility.

## Required analysis
- inspect current multi-device apply behavior
- inspect backend orchestration logic
- identify reusable selection and compatibility patterns

## Implementation tasks

### 22.1 Orchestration model
Create:

```text
docs/technical/backend/orchestration-model.md
```

The model must define:
- target selection
- capability filtering
- operation planning
- partial success handling
- skip reporting
- result summarization

### 22.2 Shared target selection UI
Create reusable multi-select targeting for:
- Lighting
- Fans
- Profiles
- LCD/media assignment

### 22.3 Compatibility-aware batch apply
Before applying:
- show compatible targets
- show incompatible targets
- show what will be skipped
- ask for confirmation if needed

### 22.4 Cross-device copy workflows
Support:
- copy lighting configuration from source to targets
- copy fan settings from source to targets
- copy profile references where meaningful

### 22.5 Result summaries
After batch actions, show:
- number of targets
- succeeded
- skipped
- failed
- reason breakdown

### 22.6 Documentation
Update relevant feature specs and backend orchestration docs.

## Acceptance criteria
- batch operations are safe and visible
- selection flows are reusable
- users get clear result summaries

---

# 20. Phase 23 — Access Model, Roles, Permissions, and Auditability

## Goal
Improve access control and operational safety for shared environments.

## Why this phase exists
As the feature surface grows, write access must become more deliberate and auditable.

## Required analysis
- inspect current authentication behavior
- inspect route-level protection
- decide whether auth remains static, reverse-proxy-backed, or becomes more internal

## Implementation tasks

### 23.1 Access model
Create:

```text
docs/technical/architecture/access-model.md
```

Define roles:
- Viewer
- Operator
- Admin

### 23.2 Route-level authorization
Add protection for:
- reads
- writes
- destructive actions
- settings
- import/export
- diagnostics operations

### 23.3 UI role awareness
The frontend must:
- hide or disable unauthorized actions
- explain why an action is unavailable
- clearly indicate read-only mode

### 23.4 Audit-friendly mutation logging
Track:
- who initiated the change
- target(s)
- operation type
- timestamp
- outcome summary

### 23.5 Documentation
Create or update:
- `docs/technical/deployment/security.md`
- `docs/functional/admin-guides/user-access.md`
- `docs/functional/admin-guides/authentication.md`

## Acceptance criteria
- role boundaries are clear
- unauthorized actions are blocked
- UI reflects permissions
- write operations are auditable

---

# 21. Phase 24 — Final Documentation Hardening and Productization Pass

## Goal
Bring the documentation, release quality, and regression readiness to a product-grade level.

## Why this phase exists
A feature-rich system without polished documentation remains hard to maintain and adopt.

## Required analysis
- compare docs to implemented behavior
- identify broken links, stale examples, duplicate docs, and terminology drift

## Implementation tasks

### 24.1 Review all feature specs
Ensure feature specs exist and match the implemented system for:
- dashboard
- devices
- lighting
- fans
- wireless
- LCD/media
- profiles
- diagnostics
- settings
- auth/roles if implemented

### 24.2 Add screenshot and diagram references
For major functional docs, add placeholders or references for:
- screenshots
- workflow diagrams
- architecture diagrams
- capability maps

### 24.3 Release-ready changelog process
Create:
- release note template
- changelog update workflow
- feature status table

### 24.4 Known limitations pages
Document:
- unsupported daemon capabilities
- unsupported device classes
- partial wireless workflows
- partial LCD workflows
- integration boundaries

### 24.5 Regression checklist
Create:

```text
docs/technical/testing/regression-checklist.md
```

Cover:
- startup
- daemon connectivity
- device listing
- lighting apply
- fan apply
- profile apply
- batch operations
- settings
- diagnostics
- auth and permissions
- import/export
- LCD/media if implemented

### 24.6 Final consistency sweep
Check:
- English-only compliance
- broken links
- route mismatches
- stale API examples
- duplicate terms
- outdated references

## Acceptance criteria
- docs are complete and structured
- release process is documented
- regression checklist exists
- the project is easier for contributors and users to understand

---

# 22. Definition of Done for This Expansion Cycle

This expansion cycle is complete when:

- the frontend has a coherent information architecture
- the UI has a reusable visual system
- devices are presented as structured inventory and workspaces
- lighting and fan workflows are significantly richer
- wireless workflows have a dedicated surface
- LCD/media workflows are exposed where supported
- profiles are richer and import/export capable
- diagnostics and settings are first-class product areas
- permissions and auditability are clearly documented or implemented
- all documentation is English-only and separated into technical and functional sections

---

# 23. Final Instruction to the Coding AI

Implement in this order:

1. documentation foundation
2. information architecture
3. device inventory and workspaces
4. lighting workbench
5. fan workbench
6. wireless workbench
7. LCD/media workbench
8. profiles/import-export
9. diagnostics/settings/update
10. batch operations
11. access model
12. final documentation hardening

Always:
- reuse daemon and backend capabilities first
- document capability gaps explicitly
- update both technical and functional docs
- keep the UI structured and scalable
- prefer safe and explicit UX over hidden automation