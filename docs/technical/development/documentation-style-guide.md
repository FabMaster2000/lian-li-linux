# Documentation Style Guide

## Purpose

This guide defines the documentation standard for all first-party project documentation.
It exists to keep the repository English-only, audience-aware, easy to navigate, and consistent across technical and functional content.

## Scope

This guide applies to:

- repository-owned Markdown files in the repository root
- all files under `docs/`
- future technical and functional documentation created during the post-MVP expansion

This guide does not rewrite upstream vendor documentation in `vendor/`, but any first-party document that references vendor behavior must still summarize it in English.

## Non-Negotiable Rules

1. All first-party documentation must be written in English only.
2. Technical and functional documentation must be stored separately:
   - `docs/technical/` for developers, maintainers, contributors, and operators
   - `docs/functional/` for end users, admins, testers, and reviewers
3. Each document must have one clear audience and one clear purpose.
4. Do not create duplicate sources of truth. If information already exists in the correct document, link to it instead of restating it.
5. Documentation must be updated in the same task or change set that modifies behavior, APIs, workflows, or operator steps.

## File Placement and Naming

- Use lower-case kebab-case filenames such as `system-overview.md` or `fan-workbench.md`.
- Use `README.md` only for index pages or section entry points.
- Keep technical reference material under the matching technical subfolder such as `architecture/`, `backend/`, `deployment/`, `testing/`, or `development/`.
- Keep user-facing workflows under `docs/functional/user-guides/`, `feature-specs/`, `admin-guides/`, or `troubleshooting/`.
- Do not keep new first-party docs in the legacy flat `docs/` root unless the file is a temporary migration artifact that is marked for removal.

## Required Document Opening

Major documents should start with:

1. One `#` heading that names the document.
2. A short paragraph that explains what the document covers.
3. A metadata block in plain Markdown bullets.

Use this opening pattern:

```md
# Document Title

Short description of what this document covers and what it does not cover.

- Status: implemented
- Audience: developers, operators
- Source of truth: backend API
- Last reviewed: 2026-03-14
- Related documents:
  - `docs/technical/backend/endpoints.md`
  - `docs/technical/architecture/system-overview.md`
```

Use plain Markdown metadata instead of YAML front matter so the files remain portable across GitHub, local editors, and static site tooling.

## Heading Conventions

- Use exactly one H1 heading per file.
- Use Title Case for headings unless a code identifier, protocol term, or file path requires another style.
- Use H2 headings for the main sections of the document.
- Use H3 headings only when a section genuinely needs subdivision.
- Keep headings specific and stable. Prefer `Error Model` over vague headings such as `Notes`.
- If a document is task-oriented, order sections from context to procedure to validation to limitations.
- If a document is reference-oriented, order sections from scope to model to examples to edge cases.

## Writing Style

- Write in direct, factual English.
- Prefer short paragraphs and explicit lists over long narrative blocks.
- State assumptions, limitations, and unsupported behavior directly.
- Prefer active voice when describing responsibilities or actions.
- Avoid historical task phrasing such as "in this phase we did X" unless the document is explicitly a changelog or roadmap.
- Use consistent terms for the same concept everywhere in the repository.

## Glossary Rules

- The canonical user-facing glossary must live in `docs/functional/glossary/terms.md`.
- Add new domain terms to the glossary before they are used broadly across multiple documents.
- Expand uncommon abbreviations on first use in each document.
- When a term has both a user-facing and implementation-facing meaning, define both clearly instead of relying on context.
- Do not redefine glossary terms in conflicting ways across documents.

## Terminology Rules

Use these canonical terms:

- `frontend`: the browser-delivered React application
- `backend`: the HTTP and WebSocket service
- `daemon`: `lianli-daemon`
- `IPC`: communication between backend clients and the daemon
- `device`: a controllable hardware unit exposed to the web stack
- `controller`: the hardware or logical controller that manages one or more devices
- `group` or `channel`: only when the underlying hardware model actually exposes that distinction
- `profile`: a reusable saved configuration bundle
- `preset`: a built-in predefined configuration
- `workbench`: a focused workflow page such as Lighting or Fans
- `desktop GUI`: the legacy native GUI, only when it must be mentioned explicitly

Terminology restrictions:

- Do not call the web frontend the `GUI` without qualification.
- Do not use `config`, `profile`, and `preset` interchangeably.
- Do not use vendor-specific marketing names as generic technical categories unless the device actually uses that name.
- Use `Wireless Sync` as the product surface name and `binding`, `rebind`, or `unbind` for the specific operations inside that surface.

## Section Expectations by Document Type

Recommended sections for technical reference documents:

- Purpose or Scope
- Audience
- Source of Truth
- Model, Contract, or Architecture
- Operational Notes or Limitations
- Related Documents

Recommended sections for user guides:

- What this page or feature does
- Prerequisites
- Step-by-step workflow
- Expected result
- Limitations or unsupported cases
- Related guides or troubleshooting

Recommended sections for troubleshooting documents:

- Symptoms
- Likely causes
- Checks to run
- Recovery steps
- Escalation path or known limitations

## Code Block Conventions

- Always use fenced code blocks with a language tag when one exists.
- Use `bash` for shell commands, `json` for JSON payloads, `text` for plain output, `rust` for Rust, `ts` or `tsx` for TypeScript, and `mermaid` for Mermaid diagrams.
- Commands must be copyable. If a command assumes a working directory, say so immediately above the code block.
- Use `text` blocks for file trees, console output, or pseudo-structured data that is not valid JSON.
- Keep examples minimal but realistic. Do not invent fields that do not exist in code or planned contracts.
- Redact secrets, tokens, personal data, hostnames, and private paths unless the path is intentionally documented as a local example.

Example:

```bash
cd frontend
npm run build
```

## API Example Conventions

Every API document or endpoint section should state:

- method and path
- purpose
- auth assumptions if applicable
- request example when the route accepts input
- success response example
- relevant error shape or failure conditions

Rules for API examples:

- Use stable placeholder values such as `device-1`, `profile-silent`, or `controller-01`.
- Keep JSON field ordering consistent with the real API where possible.
- Show only fields that matter to the example.
- If behavior is partial or planned, label it with the feature status instead of presenting it as complete.

Example section layout:

````md
## POST /api/profiles

Creates or updates a stored profile.

Status: partial

Request example:

```json
{
  "name": "Balanced Cooling",
  "targets": ["device-1"]
}
```

Success response:

```json
{
  "id": "profile-balanced",
  "name": "Balanced Cooling"
}
```
````

## Diagram Rules

- Use diagrams only when they clarify structure, ownership, sequencing, or data flow better than prose.
- Prefer Mermaid for sequence, flow, or state diagrams that will be maintained in Markdown.
- Prefer plain `text` diagrams for simple topology sketches when Mermaid would add noise.
- Every diagram must be introduced by one sentence that explains what the reader should learn from it.
- Diagram labels must use the same terminology as the surrounding documentation.
- Update diagrams in the same change that updates the described architecture or workflow.

## Screenshot Policy

- Add screenshots only when the UI layout, state, or visual distinction matters and prose would be insufficient.
- Store screenshots under `docs/assets/screenshots/` and diagrams under `docs/assets/diagrams/` when binary assets are introduced.
- Use descriptive filenames such as `lighting-workbench-pending-changes.png`.
- Add surrounding text that explains what the screenshot shows and why it matters.
- Never include secrets, personal data, private hostnames, or irrelevant desktop content.
- If a screenshot is planned but not yet available, use a short placeholder note instead of a broken image link.

## Versioning and Review Notes

- Every major document should include `Status` and `Last reviewed` metadata near the top.
- Update `Last reviewed` whenever the document is materially changed.
- If a change alters behavior, API contracts, setup steps, or UI flow, update the relevant docs in the same task.
- Roadmaps, feature specs, and changelogs may describe future work, but they must clearly separate implemented behavior from planned behavior.
- When a document points to another source of truth, say so explicitly instead of duplicating the full content.

## Deprecation Notes

- Mark deprecated documents clearly near the top of the file.
- State what replaced the document and link to the replacement.
- State whether the document is retained for history, migration, or compatibility.
- Do not silently leave stale files in place once a replacement exists.
- If a deprecated document is kept temporarily, remove procedural language that could mislead readers into treating it as current.

Recommended deprecation banner:

```md
> Status: deprecated
> Replaced by: `docs/technical/deployment/ubuntu-headless-setup.md`
> Note: retained temporarily during the documentation migration.
```

## Feature-Status Labels

Use one of these labels near the top of a document or section when status matters:

| Label | Meaning | When to use it |
| --- | --- | --- |
| `planned` | Intended design exists, but behavior is not implemented yet. | Roadmaps, future-facing feature specs, or placeholder docs. |
| `partial` | Some behavior exists, but important capabilities, edge cases, or UX flows are missing. | MVP surfaces, incomplete APIs, or staged migrations. |
| `implemented` | The documented behavior is available now and should match code or shipped UX. | Stable technical references and current user guides. |
| `deprecated` | The document or feature should no longer be used as the primary path. | Replaced workflows, legacy docs, or migration shims. |

## Links and Cross-References

- Use repository-relative paths when linking to other project documents.
- Link to the canonical source instead of copying the same explanation into multiple files.
- Keep link text descriptive. Prefer `profile storage` over `click here`.
- When a section depends on a capability gap or later phase, link to the relevant feature spec, roadmap entry, or technical limitation document.

## Author Checklist

Before merging or keeping a document:

- Confirm the document is in English.
- Confirm the file lives in the correct technical or functional section.
- Confirm the opening metadata block includes at least `Status`, `Audience`, and `Last reviewed` for major docs.
- Confirm headings follow the required structure.
- Confirm glossary terms and product terminology are consistent.
- Confirm code blocks, API examples, and diagrams follow the conventions in this guide.
- Confirm limitations, unsupported behavior, and partial implementations are called out explicitly.
- Confirm links point to the intended source-of-truth documents.
