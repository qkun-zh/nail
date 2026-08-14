# Progress log — nail → nail_new migration

Chronological record of completed work. This is history, not instructions:
the current pickup doc is `document/handoff.md`, the rules are `README.md`.
Per-slice §8.3 gate comparisons and probe findings are recorded here.

## Phase 2 — common crate (2026-08-13, commits `1c8d543`..`22a4f3c`)

9 modules (`text, name, tag, response, hash, time, pow, request, search`),
TDD red→green, **104 tests green** at `test/unit/common/<module>/tests.rs`
via `#[path]`. Owner refinements: typed lowercase `DeleteMode`, unified
`DeleteBody`, generic `err()`, panic-free `token() -> anyhow::Result`,
both-or-neither check, no tracing, RFC3339 offset formatter (#19),
`SearchHit` single-sourced (#10).

## Phase 3 slice 1 — authentication/session (2026-08-13, commit `6c063b0`)

`GET /challenge/read`, `POST /email/read?intent=...`, `POST /user/create`,
`GET /session/read`, `POST /session/delete` across the four layers;
**back 59 tests green**. Back module tree = ADR-0001; `intent` contract =
ADR-0002 (#26 closed); glossary = `document/context.md`.

§8 gate: equal-or-better — `TokenCache<E>` replaces the legacy six-file token
duplication, `EmailSender` seam, explicit `intent`, corrected error semantics
(AC4/AC5). Probe: moka's eviction listener is housekeeper-driven (eventually
consistent), same as legacy.

## Phase 3 slice 2 — user domain (2026-08-13, commits `635a53e`..`acca62a`)

`GET /user/{id}/read`, `GET /user/read`, `POST /user/{id}/update`,
`POST /user/{id}/delete`, and the `change_email`/`deregister` branches of
`POST /email/read` (stubs removed). **back 117 tests green** (common 104).

- #15: symmetric `email_hash` defaults (false everywhere); self-read errors
  surfaced. #27: idempotent deregister confirm. #25: `has_next = page <
  total_pages`.
- New `repository/transfer.rs` (recycler selection + asset transfer) and
  `repository/delete.rs` (hard delete); `role.rs` gained `user_holds_permission`
  (interim gate until Cedar in slice 6); `cache.rs` gained
  `EmailUpdateTokenEntry`/`DeregisterTokenEntry` and atomic `consume_if`
  (moka `and_compute_with`, key-serialized).
- §8.3: equal-or-better. Library facts: agdb `remove` cascades node edges
  (`remove_query.rs`); moka `and_compute_with` is key-serialized
  (`entry_selector.rs`); `Entry::value()` returns a clone. `actor_id` passed
  directly (no redundant re-authentication); `EmailMismatch` replaces the
  legacy opaque `bool`; one `send_confirmation_email` helper replaces three
  near-identical blocks.

## Phase 3 slice 3 — article + version (2026-08-13/14, `8de3490` + `6747cad`)

The owner ordered a clean rewrite of slices 1-3 under the new CRUD-only
vocabulary (README §5.2). The backend was rebuilt from the empty skeleton via
TDD (sub-agents). **back 180 tests green** (common 104), `cargo check` zero
warnings. The `_`-prefixed archive files were removed (commit `aa39cb6`); the
pre-rewrite implementation is recoverable from git (`8de3490^`).

- Covers all 19 slice 1-3 routes across the four layers. Adjudication: #6
  (version list read-open), #15, #17 (distinct `ContentHashTaken`), #18
  (`latest_version_id` in the article list), #20 (no unconditional startup
  rebuild), #21 (seekstorm count), #23 (no redundant author lookup), #25, #26,
  #27. Slice-2 deferred items done: search re-sync after rename/deregister
  (best-effort), hard-delete cascade + PDF cleanup, recycler least-loaded
  selection.
- Repository interfaces designed fresh per §4.1: typed `ArticleDraft`/
  `ArticleUpdate`/`VersionDraft`, a `SearchIndex` struct
  (`open_or_create`/`sync`/`sync_user`/`sync_all`/`read`/`close`), fresh query
  names (`owner_of`/`content_hash_owner`/`parent_article_of`/`versions_of`).
- §8.3 gate, grounded in library source + probes:
  (a) agdb serializes writes — `agdb-0.13.2/src/db.rs` documents "only single
  write operation at any one time"; `transaction_mut` commits on `Ok`/rolls
  back on `Err`; content-hash uniqueness is checked inside the txn, probed by
  `concurrent_identical_content_hashes_are_serialized_by_the_write_lock`.
  (b) seekstorm count (#21) single-sourced — `seekstorm-3.3.5/src/index.rs`:
  `current_doc_count` = "indexed − deleted", `update_document` = "delete +
  index"; `read` uses `result_count_total`; probed by
  `sync_all_and_incremental_sync_agree_on_document_count`.
  (c) PDF boundaries (`%PDF-` + `1.x`/`2.x` + `%%EOF` within last 1024 bytes,
  ≥10 bytes, ≤ max size) probed by the `infrastructure/pdf.rs` matrix.
  (d) `PdfUpload` RAII (`Received` drops temp, `Placed` drops final unless
  `keep_final()`, `Kept` persists) probed by
  `upload_places_the_pdf_and_drops_an_unkept_placed_file`.
  Probe finding: seekstorm 3.3.5 does not release RAM on `IndexArc` drop —
  `index.rs` `Close` ("Remove index from RAM") must be called manually; added
  `SearchIndex::close()` (wired into graceful shutdown, FR-7).
- Tooling note (verified 2026-08-14): the file tools work normally on `test/`
  and `code/back/src/repository/` files. Earlier claims that they failed were
  false — verified by reading `repository/comment.rs` and
  `test/unit/back/repository/delete.rs` with `read_file`.

## Phase 3 slice 4 — comment domain (2026-08-14, commit `91597bb`)

`POST /version/{id}/comments/create`, `POST /comments/{id}/replies/create`,
`GET /version/{id}/comments/read`, `POST /comment/{id}/update`,
`POST /comment/{id}/delete` across the four layers. **back 205 tests green**
(common 104), `cargo check` zero warnings.

- #2 (`DeleteBody` mode contract), #16 (invalid comment id → 400), #3 (no
  per-comment pre-check; backend 403 authoritative). `read_comments` pages by
  top-level comments with depth-bounded reply trees (max 64); batch user-name
  lookup; version-level `is_author` only.
- Bug fix (caught by the new hard-delete subtree test):
  `delete_comment_tree_in_txn` traversed `comment_to_comment` in the wrong
  direction (`.from(comment)` + `edge.to` walks up toward the parent), so
  replies were never cascade-deleted. Corrected to `.to(comment)` +
  `edge.from` — also fixed the slice 3 article/version/user cascades.
- Test infra: `SearchIndex::open_or_create_with_segments`; the harness builds a
  4-segment index instead of 2048, stopping an OOM (production unchanged).
- §8.3: typed `CommentTreeItem` replaces legacy `serde_json::Value` rows;
  `map_create_comment_error(is_reply)`; batch `read_user_names`; depth check
  and writes inside the graph write transaction.

## Phase 3 slice 5 — download/PDF (2026-08-14, commit `1d29a6a`)

`GET /article/{id}/version/{version_id}/content/read` across the four layers.
**back 224 tests green** (common 104), `cargo check` zero warnings.

- #1: mint returns `.../content/read?token={token}`; consume passes
  `params.token` (the legacy passed the path `version_id` as the token and
  discarded the minted token — both halves of the broken chain fixed). Token
  single-use (atomic `consume_if`), TTL `download_token_ttl_seconds` (60 s),
  bound to the minting user (400 "download token is bound to another account").
  #28: served filename always the hash-derived `<hash>.pdf`; no original
  filename stored. #29: `download=1|true` returns the `{url}` envelope from the
  otherwise-binary route (deliberate contract).
- New: `logic/download.rs` (`mint_download_token`/`consume_download_token`/
  `resolve_version_pdf_path`), `interface/content.rs` (`read_content` +
  streaming `ReaderStream`), `DownloadTokenEntry` + `download` cache, config
  `download_token_ttl_seconds`.
- §8.3: `DownloadTokenEntry` rides the generic `TokenCache<E>` (legacy's
  dedicated `repo/token/download.rs` gone); streaming with no in-memory read
  and no reverse-index bookkeeping.

## Phase 3 slice 6 — role/authorization, Cedar (2026-08-14, commit `34f4dfd`)

The five role routes + Cedar authorization across the four layers.
**back 240 tests green** (common 104), `cargo check` zero warnings.

- Cedar landed: `cedar-policy 4.12.0`, `infrastructure/cedar.rs` (cached
  `PolicySet` via `OnceLock` + `decide()`), `infrastructure/cedar/{schema,
  policy}.cedar`, `repository/authorization.rs` (principal/resource assembly,
  comment→version→article chain). `schema.cedar` = 7 entity types (#5 removed
  `Visibility`) + 16 actions; `policy.cedar` = 5 policies, policy 2 rewritten
  to read-open.
- #7: `member_count` real count (`read_role_members().len()`). #8: `delete_role`
  protects all `REQUIRED_ROLES`. #9: duplicate role → 400 "role already
  exists" (verdict overrides FR-47's idempotent 201).
- `logic/authorize.rs` converged to Cedar: `authorize`/`authorize_or`/
  `authorize_create`/`is_allowed`/`is_author` (FR-54). Transitional
  `require_permission`/`require_owner_or_permission_for_*`/`is_article_author`
  gone. `read_roles` replaces the forbidden `list_roles`.
- §8.3: engine/assembly/gate split across infrastructure/repository/logic;
  `RoleView` single-sourced; `PolicySet` parsed once via `OnceLock` (legacy
  re-parsed per request).
- ⚠️ Slice 6 initially converged to legacy policy 1, which excludes
  `Version::Update`/`Version::Delete`/`Comment::Update` from the owner bypass.
  The owner adjudicated this design wrong (**#33**, 2026-08-14) — see the
  current handoff; the policy-1 amendment is a pending task.

## #33 — owner-bypass amendment (2026-08-14, commit `efe8cfe`)

Owner ruling: legacy policy 1's exclusion of `Version::Update`/
`Version::Delete`/`Comment::Update` from the owner bypass is wrong (contradicts
FR-20/FR-21). Policy 1 in `infrastructure/cedar/policy.cedar` now includes the
three actions. `repository/authorization.rs` was verified correct —
`Comment.owner` = comment author, `Version.owner` = article owner — no assembly
fix needed. Slice 6's 5 denial tests flipped to owner-allow; a
comment-author ≠ article-owner assembly test added. Member seed grants NOT
widened (non-owner-scoped).

## Phase 3 slice 7 — config/email/infrastructure (2026-08-14, commit `06c72f4`)

Final backend slice. **back 245 tests green** (common 104), `cargo check` zero
warnings.

- `/config/read` returns the typed `common::response::RuntimeLimits` DTO (11
  fields), not `json!`. Config validation matrix: empty path / difficulty 0 or
  >10000 / zero ttls+capacities / zero content limits / text_field > pdf /
  zero pagination limits → `AppConfig::load` fails → `main` appends to
  `startup-errors.log` and exits 1.
- #13: `db_namespace`/`db_database`/`max_id_filter_count` confirmed absent
  (never entered `ServerConfig`/`server.toml`). #19: timezone is config
  (`timezone_offset_seconds`, whole minutes ±23:59), served via `/config/read`,
  consumed by logging (`OffsetTime`) + search; no hardcoded `+08:00` anywhere.
  #22: multipart read-then-validate (K) — `PdfStreamGuard` streaming + body
  bound already in place. #26: email intent converged (ADR-0002). #32: e2e
  flag + test tree → Phase 5.
- `main.rs` bootstrap: config load → fail-fast (`startup-errors.log`, exit 1)
  → `logging::init` + `prune_loop` (per-minute rotation + retention prune, 2
  new prune tests) → `run_server` (graceful shutdown closes the search index,
  verified). `max_comment_body_chars` moved from a constant into config and
  wired into `logic/comment.rs`.
- §8.3: typed `RuntimeLimits` replaces the legacy `api/meta.rs` `json!` (which
  also leaked the dead `max_page`); logging drops `chrono::Local` for the
  `time` crate + config offset (#19); config sheds dead/backend-internal
  fields.

## Phase 4 — frontend migration (2026-08-14, commits `46fa23e`..`99b83c7`)

Leptos CSR frontend, layered per README §4.2 (main → router → page → request
→ infrastructure). **front 61 unit tests green**, `cargo check --target
wasm32-unknown-unknown` + host `cargo check` zero warnings; back 245 + common
108 green (no regression). Module layout: `infrastructure` (config/limits/
storage/pow), `request` (error/url/envelope/session/http/pow/auth/user/article/
version/comment/download), `page` (session_gate/author_gate/notify/pagination/
validation/draft/time_format + public/private sections), `router` (FR-60 route
table). All 21 reached routes exercised; responses deserialize the typed
`common::response` DTOs. Adjudication: #14 (English UI), #24 (page size from
config), #25 (server `has_next`), #3 (no per-comment pre-check), #12 (no
`/private/email/check`), #4 (PoW in-wasm, no worker), #19 (timezone from
limits).

## Personnel history

- Agent A/B: Phase 2 (common) + slice 1.
- Agent C: slice 2 (user domain), handoff `811e3dd`.
- Agent D: slice 3 (article + version; owner-ordered CRUD-vocabulary rewrite).
- Agent E: slice 4 (comment domain).
- Agent F: slice 5 (download/PDF) + slice 6 (role/authorization, Cedar).
- Agent G: #33 owner-bypass patch (`efe8cfe`) + slice 7
  (config/email/infrastructure, `06c72f4`) — Phase 3 backend complete.
- Agent I: Phase 4 (frontend) — complete.

## Phase 5 - tests + e2e + cleanup (2026-08-14, in progress)

### Keep-behavior refactors (commit 06d58a3)

Applied the two remaining PRD [keep-behavior] improvements: (a) a single
logic/pagination.rs clamp_page_limit now serves all five route handlers
(was duplicated in interface/comment+role+user+version and logic/article, with
two duplicated MAX_PAGE_SIZE/MAX_PAGE constant pairs); (b)
logic/error.rs::database_error centralizes the 42 duplicated
"database query failed: {error}" strings across 8 logic files. Behavior
preserved; **back 245 tests green**, zero warnings.

### Repository + Cedar test fill (commit 1bdd644)

Added 12 tests (+484/-5, 4 files): recycler selection (least-loaded, tie-break
by larger id, author-exclude, no-recycler), tag orphan cleanup, role tag-scope
apply/read/remove, and six Cedar-matrix cells (global-role grant,
non-intersecting-scope deny, admin-console wrong-resource deny,
authorize_create deny, version-owner, read-open non-owner). **back 257 tests
green**, zero warnings.

### Bug fix: role tag removal (commit 6365170)

A T1 probe found repository::role::remove_tag_from_role resolved the tag by
its business-id alias tag:{uuid} instead of by name, so the FR-50
tags.remove API path 500'd on every tag removal. Fixed to resolve by the
KEY_TAG_NAME index (mirroring apply_tag_to_role); the repository test was
corrected to assert the name-based contract. **back 257 tests green**, zero
warnings.

Next: HTTP/API branch coverage (content domain, then identity/admin), e2e (#32,
owner strategy pending), final dead-code cleanup.
