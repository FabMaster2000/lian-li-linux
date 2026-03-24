# Technical Documentation

This index is the entry point for technical project documentation.
It is intended for readers who need implementation, architecture, deployment, testing, or maintenance details.

- Status: implemented
- Audience: contributors, maintainers, operators
- Source of truth: this index for technical documentation entry points
- Last reviewed: 2026-03-15
- Related documents:
  - `development/doc-audit.md`
  - `development/documentation-style-guide.md`
  - `architecture/source-of-truth.md`

## Target Audience

- Developers working on the frontend, backend, daemon integration, or shared types
- Maintainers reviewing architecture, ownership, and operational boundaries
- Operators who need deployment, configuration, validation, or observability guidance

## What Belongs Here

- Architecture and domain-model documentation
- Backend and frontend technical references
- Deployment, configuration, and security guides
- Testing strategy, manual validation, and regression guidance
- Development workflow, code-structure, and documentation process guidance
- ADRs and other technical decision records

## How to Navigate

Start with these foundation documents:

1. [System Overview](architecture/system-overview.md)
2. [Backend Architecture](architecture/backend-architecture.md)
3. [Frontend Architecture](architecture/frontend-architecture.md)
4. [Daemon Integration](architecture/daemon-integration.md)
5. [Domain Model](architecture/domain-model.md)
6. [Source of Truth Matrix](architecture/source-of-truth.md)
7. [Documentation Style Guide](development/documentation-style-guide.md)
8. [Documentation Audit](development/doc-audit.md)

Use the remaining flat technical references as interim documents while migration is still in progress:

- [API](../api.md)
- [Deployment](../deployment.md)
- [Profile Model](../profile-model.md)
- [Profile Storage](../profile-storage.md)
- [Event Model](../event-model.md)
- [Backend Config](../backend-config.md)
- [Config Analysis](../config-analysis.md)
- [Frontend State Model](../frontend-state-model.md)
- [Frontend State Strategy](../frontend-state-strategy.md)

Phase 14 frontend structure documents:

- [Current Information Architecture](frontend/current-information-architecture.md)
- [Target Information Architecture](frontend/target-information-architecture.md)
- [Design System](frontend/design-system.md)

Phase 17 fan-control documents:

- [Fan Capability Matrix](architecture/fan-capability-matrix.md)
- [Fan Workbench Model](architecture/fan-workbench-model.md)
- [Fan Curve API](backend/fan-curve-api.md)

Target technical areas in the final structure:

- `architecture/`
- `frontend/`
- `backend/`
- `deployment/`
- `testing/`
- `development/`
- `adr/`

## Current Migration Status

- `development/` currently contains the migration audit and the documentation standard.
- `architecture/` now contains the system, backend, frontend, daemon-integration, domain-model, source-of-truth, and device-ordering foundation documents.
- The removed legacy redirect files should not be recreated.
- Several technical migration inputs still live in the flat `docs/` root and should be treated as transition-era references.

## How to Contribute

- Create new technical docs in the correct target subfolder instead of the flat `docs/` root.
- Follow the opening metadata, heading, terminology, and example rules in [Documentation Style Guide](development/documentation-style-guide.md).
- Before adding a new reference document, confirm ownership in [Source of Truth Matrix](architecture/source-of-truth.md).
- When migrating a legacy technical doc, update or consult [Documentation Audit](development/doc-audit.md) so the migration path stays explicit.
- Prefer linking to the canonical technical document instead of duplicating the same explanation across backend, frontend, and deployment docs.

