# reference — Reconstruction Overview

| Setting | Value |
| --- | --- |
| Mode | `redesign` |
| Level | `complex` |
| Fidelity | `describe` |
| TDD | `on` (build test-first) |
| Generated with | `reconstruct@2.17.0` |

## Product summary

`nail` is a passwordless email-verification blog platform: visitors register and
log in purely through emailed one-time tokens (guarded by proof-of-work), create
articles with versioned PDF attachments (semver-ordered versions, content-hash
deduplication), comment in a depth-limited tree, and search articles by full
text across title/summary/author/comment/note/tag. Access control is a
role-based Cedar authorization layer (admin / member / recycler / custom roles
with scope tags) layered over ownership rules, and the whole system is fronted
by a pingap reverse proxy that enforces rate limits and body-size limits.

The core value: a self-hosted, bot-resistant, permission-controlled publishing
and document-versioning platform whose identity model needs no passwords — the
cost of spam/abuse is paid in PoW compute and email proof, and content access is
auditable through explicit Cedar policy.

## Tech stack

- **Primary language:** Rust (3 crates: `common` / `back` / `front`)
- **Backend:** axum; embeddings: agdb (graph db), seekstorm (full-text search), moka (cache); cedar-policy (authorization); pso-vdf (MinRoot PoW); ascon-xof128 (hashing)
- **Frontend:** Leptos CSR (wasm32), built with trunk; no CSS by design
- **Proxy:** pingap (static assets + `/api` reverse proxy; rate limits, body limits, access logs)
- **Tooling:** cargo, rustup (wasm32-unknown-unknown target); `comment-stripper-rs` for doc stripping

## Metrics

- Files analyzed: **135** (16294 lines)
- Features/modules: **2**
- Routes: **31** (verified against `api.rs`)
- Locales: **0**
- Tracked env vars: **0**

## Feature index

- [`01-project-setup`](../features/01-project-setup/PRD.md) — **Project Setup & Tooling**: 3 configuration/tooling file(s): build, lint, env, CI.
- [`02-code`](../features/02-code/PRD.md) — **Code**: Groups 120 file(s).

## How to use this output

1. Read `architecture/ARCHITECTURE.md` for the overall shape, then `architecture/INTERFACES.md` (the full interface surface) and `architecture/DATA-MODEL.md` (entities & relations).
2. Rebuild feature by feature using each `features/<slug>/PRD.md`, in the order listed in `REBUILD.md`.
3. Use `data/` (translations, schema, config) and — when present — `source/` as ground truth.

## Redesign note

This run is in **redesign** mode: preserve every feature's behavior and logic,
but restructure into the target layered architecture defined in the `nail_new`
README (back: interface → logic → repository → infrastructure; front:
router → page → request → infrastructure; common: shared structures). The
proposed structure and per-module rationale are in `architecture/ARCHITECTURE.md`.
The feature PRD `02-code` carries a `notes` section listing ~32 source-level
inconsistencies and suspected bugs for human adjudication before the rebuild.
