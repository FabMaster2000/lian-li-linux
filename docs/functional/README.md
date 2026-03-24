# Functional Documentation

This index is the entry point for user-facing and workflow-facing documentation.
It is intended for readers who want to understand how to use, operate, review, or troubleshoot the product rather than how the code is implemented.

- Status: partial
- Audience: users, admins, testers, reviewers
- Source of truth: this index for functional documentation entry points
- Last reviewed: 2026-03-15
- Related documents:
  - `../technical/development/documentation-style-guide.md`
  - `../technical/architecture/source-of-truth.md`
  - `../../README.md`

## Target Audience

- End users who need feature usage guidance
- Homelab operators and admins who need operational workflows
- Testers and reviewers who need expected behavior, feature scope, and troubleshooting references

## What Belongs Here

- User guides for pages and workbenches
- Admin guides for authentication, deployment basics, backups, and access
- Feature specifications that describe intended user-visible behavior
- Troubleshooting guides for common operational and device issues
- Glossary entries for shared product terms
- Roadmap and changelog information intended for broader readers

## How to Navigate

This tree is the canonical home for functional documentation even though the migration is still at an early stage.

Use these target sections as the navigation model:

- `user-guides/`
- `admin-guides/`
- `feature-specs/`
- `troubleshooting/`
- `glossary/`
- `release-notes/`

Current practical entry points while the functional tree is still being filled:

- [Getting Started](user-guides/getting-started.md)
- [Dashboard](user-guides/dashboard.md)
- [Devices](user-guides/devices.md)
- [Lighting](user-guides/lighting.md)
- [Fans](user-guides/fans.md)
- [Fan Curves](user-guides/fan-curves.md)
- [Profiles](user-guides/profiles.md)
- [UI Foundation](feature-specs/ui-foundation.md)
- [Fan Workbench](feature-specs/fan-workbench.md)
- [Top-level Documentation Index](../README.md) for migration-aware navigation
- [Repository README](../../README.md) for product overview and supported-device context

## Current Migration Status

- The functional tree now contains the first user-guide foundation for the current dashboard, devices, lighting, fans, and profiles routes, plus a dedicated feature spec for the Phase 15 device dashboard and topology experience.
- Functional docs are still incomplete for later-phase areas such as diagnostics, wireless, LCD/media, and admin workflows.
- Where implementation is ahead of functional guidance, technical docs may still be needed as support references, but they do not replace final user-facing documentation.

## How to Contribute

- Create new user-facing content under `docs/functional/`, not in the flat `docs/` root.
- Use plain, task-oriented English and follow the rules in [Documentation Style Guide](../technical/development/documentation-style-guide.md).
- Check [Source of Truth Matrix](../technical/architecture/source-of-truth.md) before duplicating workflow or status information that already has a canonical home.
- Mark incomplete user-facing content with the correct status label such as `planned` or `partial`.
- When a functional document depends on unfinished implementation, state the limitation clearly instead of presenting planned behavior as already available.

