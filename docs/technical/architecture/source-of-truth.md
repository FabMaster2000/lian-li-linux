# Source of Truth

This document defines where the authoritative truth lives for each major concern in the Lian Li Linux web controller stack.
It exists to prevent duplicate ownership across the daemon, backend, frontend, and documentation layers.

- Status: implemented
- Audience: contributors, maintainers, operators
- Source of truth: this document
- Last reviewed: 2026-03-15
- Related documents:
  - `docs/technical/development/documentation-style-guide.md`
  - `docs/technical/development/doc-audit.md`

## Decision Rules

- Every concern should have one primary source of truth.
- Supporting documents may explain or contextualize a concern, but they do not replace the primary source.
- If implemented behavior and documentation disagree, the owning implementation layer is the temporary truth until the documentation is corrected.
- If a feature is planned but not implemented, the approved technical or functional specification is the truth until code exists.
- If a target document in the new structure does not exist yet, the matrix still names that target location so later documentation work converges on one canonical home.

## Source-of-Truth Matrix

| Concern | Source of Truth | Notes |
| --- | --- | --- |
| Hardware transport behavior | daemon transport and device layer, documented in `docs/technical/architecture/daemon-integration.md` | Current practical implementation lives in `crates/lianli-transport/`, `crates/lianli-devices/`, and `lianli-daemon`; docs should describe but not redefine transport behavior. |
| Device discovery, identity, and capability modeling | `docs/technical/architecture/domain-model.md` backed by daemon and shared types | The daemon/shared model remains authoritative if the architecture document lags behind implementation. |
| Daemon IPC request and response semantics | `docs/technical/architecture/ipc-mapping.md` backed by shared IPC types | Until the new document exists, `docs/ipc-protocol.md` and `crates/lianli-shared/src/ipc.rs` are the practical reference. |
| Backend HTTP API contract | `docs/technical/backend/api-overview.md` and `docs/technical/backend/endpoints.md` | Until migration is complete, `docs/api.md`, backend route handlers, and backend tests define the active contract. |
| Backend error semantics | `docs/technical/backend/error-model.md` | If documentation lags, `backend/src/errors/` and route-level tests are authoritative for current behavior. |
| WebSocket event contract | `docs/technical/backend/websocket-events.md` | Until migrated, `docs/event-model.md` and backend event code are the active reference. |
| Backend orchestration and batch-apply semantics | `docs/technical/backend/orchestration-model.md` | This becomes the canonical reference once multi-device orchestration is expanded beyond the MVP behavior. |
| Backend profile persistence format and storage location | `docs/technical/backend/profile-storage.md` | Until migrated, `docs/profile-storage.md` plus backend storage/config code remain the current reference. |
| Import and export payload semantics | `docs/technical/backend/import-export-model.md` | This is the future canonical location once profile import/export is implemented. |
| Frontend route structure and page ownership | `docs/technical/frontend/route-map.md` and `docs/technical/frontend/current-information-architecture.md` | Until those documents exist, the router and page structure in `frontend/src/app/` and `frontend/src/pages/` are the active reference. |
| Frontend visual tokens, layout primitives, and shared UI patterns | `docs/technical/frontend/design-system.md` | Until created, the living implementation in `frontend/src/components/` and theme code is the temporary truth. |
| Frontend state-management rules | `docs/technical/frontend/state-management.md` | Until migration is complete, `docs/frontend-state-model.md`, `docs/frontend-state-strategy.md`, and the frontend hooks/state code are the current reference. |
| Page-level user workflows | `docs/functional/user-guides/` and `docs/functional/feature-specs/` | Functional docs define intended user behavior; if implementation differs, the mismatch must be fixed or documented explicitly. |
| Deployment topology, setup, and operations | `docs/technical/deployment/` | Until migration is complete, `docs/deployment.md` and backend/daemon config docs are the active references. |
| Runtime configuration defaults and supported overrides | owning component config module, documented under `docs/technical/deployment/configuration.md` | Backend defaults live in backend config code; daemon defaults live in daemon config code; docs must summarize without inventing values. |
| Diagnostics, logging, and observability behavior | `docs/technical/backend/observability.md` | Until that document exists, backend logging/event code and deployment notes are the practical reference. |
| Profile semantics, scope, and compatibility rules | `docs/technical/architecture/profile-model-v2.md` plus `docs/functional/feature-specs/profile-system.md` | Technical docs define structure and semantics; functional docs define user-facing workflow and expectations. |
| Lighting, fan, wireless, and LCD capability support | the respective capability matrix documents under `docs/technical/architecture/` | If a matrix is missing, daemon and backend implementation define current support until the matrix is created. |
| Access roles, authorization boundaries, and auditability rules | `docs/technical/architecture/access-model.md` and `docs/technical/deployment/security.md` | Until implemented, current backend auth behavior and deployment guidance remain the practical reference. |
| Documentation structure, writing rules, and status labels | `docs/technical/development/documentation-style-guide.md` | This guide is the canonical documentation standard for first-party Markdown. |
| Documentation migration and legacy-doc disposition | `docs/technical/development/doc-audit.md` | The audit controls move, merge, rewrite, deprecate, and delete decisions during the migration. |
| Product roadmap and future work sequencing | `docs/functional/release-notes/roadmap.md` | Until created, `More_Tasks.md` remains the planning reference but should be migrated out of task-document form. |
| Release history and shipped change tracking | `docs/functional/release-notes/changelog.md` | Until created, commit history and release artifacts are the only reliable historical record. |

## Conflict Resolution

Use these tie-breakers when a reader finds conflicting information:

1. For hardware behavior, daemon-side implementation wins.
2. For backend API behavior, backend code and backend tests win until the public docs are corrected.
3. For frontend rendering and navigation behavior, shipped frontend code wins until the frontend architecture docs are corrected.
4. For user instructions, the functional documentation should match shipped behavior; if it does not, update the docs or mark the feature as partial.
5. For planned work, approved feature specs and roadmap entries win over informal notes or stale task files.

## Migration Notes

- Several canonical target documents named in this matrix do not exist yet because the documentation overhaul is still in progress.
- During the migration, existing flat `docs/*.md` files remain usable inputs, but they should not become long-term parallel sources once their target documents exist.
- New documentation should already use the target source-of-truth locations named in this matrix.
