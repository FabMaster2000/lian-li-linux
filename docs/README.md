# Documentation

This index is the entry point for the project documentation.
Use it to choose the correct documentation area before reading deeper technical or functional material.

- Status: implemented
- Audience: contributors, maintainers, operators, users, reviewers
- Source of truth: this index for repository documentation entry points
- Last reviewed: 2026-03-15
- Related documents:
  - `technical/README.md`
  - `functional/README.md`
  - `technical/development/documentation-style-guide.md`

## Target Audience

- Contributors and maintainers who need architecture, API, deployment, and development references
- Operators who need setup, runtime, and troubleshooting guidance
- Users, testers, and reviewers who need workflow-oriented and feature-oriented documentation

## What Belongs Here

- Section indexes that route readers to the correct documentation area
- Migration guidance while the repository moves from a flat `docs/` structure to separated technical and functional trees
- High-level orientation for readers who do not yet know which document category they need

## How to Navigate

Start with one of these entry points:

1. [Technical Documentation](technical/README.md) for architecture, APIs, deployment, testing, and development practices
2. [Functional Documentation](functional/README.md) for user guides, feature behavior, admin guidance, troubleshooting, glossary, and release notes

Foundational migration documents:

- [System Overview](technical/architecture/system-overview.md)
- [Backend Architecture](technical/architecture/backend-architecture.md)
- [Frontend Architecture](technical/architecture/frontend-architecture.md)
- [Daemon Integration](technical/architecture/daemon-integration.md)
- [Domain Model](technical/architecture/domain-model.md)
- [Documentation Audit](technical/development/doc-audit.md)
- [Documentation Style Guide](technical/development/documentation-style-guide.md)
- [Source of Truth Matrix](technical/architecture/source-of-truth.md)

Some flat technical references still exist during the migration.
If you need current implementation details before the new target documents are filled out, start with:

- [API](api.md)
- [Deployment](deployment.md)
- [Profile Model](profile-model.md)
- [Profile Storage](profile-storage.md)
- [Event Model](event-model.md)

## Current Migration Status

- `docs/technical/` now contains the phase-foundation architecture and documentation-governance documents.
- `docs/functional/` now contains the initial user-guide foundation for the current MVP-plus routes.
- The most obvious legacy redirect files have been removed.
- Remaining flat `docs/*.md` files should be treated as technical migration inputs until they are moved into the separated structure.

## How to Contribute

- Put new technical content under `docs/technical/`.
- Put new user-facing or reviewer-facing content under `docs/functional/`.
- Follow the rules in [Documentation Style Guide](technical/development/documentation-style-guide.md).
- Check [Source of Truth Matrix](technical/architecture/source-of-truth.md) before creating a new document to avoid duplicate ownership.
- If you touch a legacy flat `docs/*.md` file, either migrate its content to the correct target location or record the migration intent in the documentation audit.
