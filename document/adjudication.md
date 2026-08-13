# Adjudication Log — reconstruction notes

One row per decision on the source-level inconsistencies found by the
reconstruction enrichment (`features/02-code/PRD.md` notes, 32 items).
Verdicts are given by the project owner one at a time; the migration
implements them.

Verdict legend: **F** = fix/implement in nail_new · **R** = remove dead code ·
**K** = keep behavior.

| # | Issue (short) | Verdict | Details |
| --- | --- | --- | --- |
| 1 | PDF download mint+consume chain broken (mint return discarded; fabricated tokenless URL; consume passes path version_id as token → always 400) | **F** | Restore the design contract: mint returns `.../content/read?token={token}`; consume branch passes `params.token`; keep single-use, TTL 60s, token bound to user. Drop the dead `/api/article/download` URL shape. Inline PDF view unaffected. |
| 2 | Frontend article/comment delete sends no `mode` → backend always 400 | **F** | Unify on `DeleteBody` in common (drop empty `DeleteArticleRequest`/`DeleteCommentRequest`); frontend sends `{"mode":"transfer"}`; backend unchanged. No PoW (session + permission gate per design). |
| 3 | Per-comment `is_author` dead; comment delete gate denies the author | **F (A1)** | Frontend drops the per-comment pre-check gate (`check_comment_is_author`/comment branch of `check_is_author`); delete button shown without author pre-check, backend 403/400 is authoritative. `handle_is_author` comment branch dies with it. Top-level (version-level) `is_author` stays. |
| 4 | `static/pow-worker.js` dead; main-thread VDF blocks UI | **R** | Delete the worker file and its protocol. Main-thread VDF blocking is a known limitation; revisit only if UX demands it. |
| 5 | `visibility` inert: never set, never filtered | **R (delete visibility)** | Remove the visibility attribute from the agdb schema, the Cedar schema/entities, and `policy.cedar` policy 2. IMPLEMENTATION NOTE: since nothing ever set visibility (de-facto always public, reads are session-gated), policy 2 must be rewritten to keep the de-facto behavior: any authenticated principal may read article/version/comment (owner/role/admin rules unchanged). Confirm this read-open semantics at migration time. |
| 6 | Version list ungated vs gated single-version read | **F** | Gate `GET /article/{id}/version/read` with `Version::Read` for consistency. |
| 7 | `member_count` hardcoded 0 in `/role/read` | **F** | Compute membership (agdb count) or drop the field if the frontend does not use it. |
| 8 | Role delete guards only admin/recycler; `member` role deletable | **F** | Protect all REQUIRED_ROLES (admin/recycler/member) from delete, aligning with the update-side protection. |
| 9 | Role create idempotent; duplicate create reports 201 | **F** | Duplicate role name → 400 "role already exists". |
| 10 | Unused common response structs / duplicated SearchHit | **R** | Remove the unused typed response structs; make SearchHit single-sourced in common. |
| 11 | Unused common request structs | **R** | Remove them: the two empty delete structs from #2, plus `CheckEmailRequest` (no route uses it — the frontend link is removed by #12), `EmailUpdateSendRequest` (superseded by `EmailReadRequest`), `EmailUpdateConfirmRequest` (superseded by the `UserUpdateRequest` token pair), `VerifySessionRequest` (route is GET + query), `AuthorCheckRequest` (route uses the `check_if_is_author?` query param). |
| 12 | Dead frontend link `/private/email/check` | **R** | Remove the link (no route, no page, no API). |
| 13 | Dead config: `db_namespace`/`db_database`/`max_id_filter_count` | **R** | Remove from `server.toml` and `conf.rs` parsing. |
| 14 | Hardcoded Chinese search-range labels (and generally non-English UI strings) | **F** | No Chinese anywhere (code, docs, UI strings, comments). Rewrite all user-facing strings in English. |
| 15 | `/user/{id}/read` asymmetric email_hash defaults; swallowed self-read errors | **F** | `email_hash` defaults false everywhere; admin passes `?email_hash=true` explicitly; self-read errors are surfaced, not swallowed. |
| 16 | Non-uuidv7 comment id → 500 in `read_comments` | **F** | Validate the id first; invalid → 400. |
| 17 | Duplicate content-hash mapped to `TitleAlreadyExists` variant | **F** | Introduce a distinct variant aligned with `CreateVersionError::ContentHashExists`. |
| 18 | `read_article` drops visibility/latest_version_id fields | **F** | Align with actual consumption: visibility gone per #5; `latest_version_id` returned where consumed. |
| 19 | Hardcoded `+08:00` timestamps (backend + frontend) | **F** | Timezone becomes config (backend toml + served via `/config/read`; frontend renders with it). |
| 20 | Full search index rebuild on every startup | **F** | Remove the unconditional startup rebuild; build the index only when absent (first boot), maintain incrementally at write time. |
| 21 | Two sources of truth for totals (agdb vs seekstorm) | **F** | Seekstorm count bug is fixed upstream; use the seekstorm count consistently (drop the empty-q agdb-count workaround). |
| 22 | Multipart text fields read to 1MiB before validation | **K** | Body limit bounds the DoS surface; keep the simple read-then-validate approach. |
| 23 | `update_article` requires author node existence despite gate | **R** | Drop the redundant author lookup (authorization already proved identity). |
| 24 | Frontend hardcoded page-size divisor 8 | **F** | Use the limit from `/config/read`. |
| 25 | `has_next` computed from total + returned page length | **F** | Compute uniformly as `page < total_pages`. |
| 26 | `/email/read` three-way semantics decided by session validity | **F** | Disambiguate with an explicit query parameter: `POST /email/read?intent=authenticate\|change_email\|deregister`. Session-validity inference removed. CONFIRMED by owner (2026-08-13): `intent` is a **query parameter**, not a body field — `EmailReadRequest` keeps `{pow?, old_email_pow?, new_email_pow?}`; a shared `EmailReadIntent` enum may live in common for frontend URL building / backend query parsing. Closed — no Phase 3 revisit needed. |
| 27 | Deregister confirm 200 when token missing + user gone | **K** | Idempotent delete semantics; treat as already-deregistered. |
| 28 | Served filename always `hash.pdf` | **K** | Hash-based naming is the desired contract; do NOT store the original filename. Simplify `sanitize_attachment_filename` accordingly. |
| 29 | Mint branch returns JSON from a binary-stream route | **K** | Contract settled with #1: `content/read?download=1` returns `{url}`; document it. |
| 30 | No frontend UI for admin/role management endpoints | **K** | Admin UI deferred (out of the migration scope); backend endpoints stay. |
| 31 | Admin list returns cleartext email hashes | **K** | Deliberate admin capability (matches auth-cache hashes); document it. |
| 32 | e2e scaffolding referenced but absent | **K** | The nail_new test tree is rebuilt from scratch (README §12 + TDD); e2e strategy decided at Phase 5. |

## Owner confirmations (2026-08-13) — Phase 2 (common crate)

Decisions approved for the `nail_common` build; the migrating agent implements them:

- Envelope `ResponseEnvelope<T> {code: u16, data: Option<T>, message: String}`, camelCase, `data: null` on errors; `ok()`/`err()` constructors, with **`err()` generic over `T`** (not restricted to `serde_json::Value`).
- `DeleteMode` serializes **lowercase** (`#[serde(rename_all = "lowercase")]`): wire values `"transfer"`/`"hard"`; round-trip test asserts the wire spelling.
- Common module list (9, replacing `xxx`/`yyy`/`zzz`): `text`, `name`, `tag`, `response`, `hash`, `time`, `pow`, `request`, `search`. Module list confirmed; build order dependency-topological, pure modules first.
- `hash::token()` is **panic-free**: returns a `Result` (the customized-CXOF init error is propagated), no `expect`.
- `EmailReadRequest` both-or-neither invariant for `old_email_pow`/`new_email_pow`: enforced via a pure check in `common::request` (back maps false → 400); tested in the request slice.
- `tracing` is **not** a common dependency; `pow` verification failures are silent at the common level and logged by the back interface layer.
- `uuid` features: `serde` + `v7` (the `time` module needs `get_timestamp()`). `serde_json` is a dev-dependency only (round-trip tests).
- Phase 2 tests live at `test/unit/common/<module>/tests.rs`, wired via `#[path]` from each module (matches the legacy convention and the skeleton's top-level `test/`).
- `UpdateVersionNoteRequest` is backend-only → lives in `back::interface`, not common (frontend never sends it).
- `hash` stays in common (single source of the ascon scheme); `TagRef` stays in `common::tag`; `pow::verify -> bool`; `pow::prove -> anyhow::Result`; difficulty = server config `pow_difficulty_iterations`, no constant in common; `bin/prove.rs` CLI helper skipped (revisit at Phase 5 if needed).
