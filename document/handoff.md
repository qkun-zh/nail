# Handoff — nail → nail_new migration

Status of the migration plan, current state, and the remaining steps. A
fresh agent can pick up from here. The project owner adjudicates design
decisions; the migrating agent implements per this document and the
referenced artifacts. All rules live in `nail_new/README.md` (the
constitution); this document records state and process only.

## Current state (facts, verified)

- `nail` (legacy) is frozen at commit `ca59215`; `document/reference/` is the
  comment-stripped snapshot — **untrusted legacy code** (32 adjudicated
  defects, `document/adjudication.md`); regenerable via
  `git -C nail archive HEAD` + `comment-stripper-rs --strip-docs`.
- `nail_new` repo (`main`): skeleton per README §3; `document/reconstruction/`
  (reconstruct@2.17.0, fully enriched, `--check` PASSES); PRD
  `features/02-code/PRD.md` (zero unresolved callouts); `architecture/`
  INTERFACES.md (31 routes, verified), DATA-MODEL.md (37 entities, 24 enums),
  ARCHITECTURE.md (old→new mapping).
- Tooling: node v22; `/home/qkun/reconstruct-tool` (v2.17.0);
  `comment-stripper-rs` in `~/.cargo/bin`; wasm32-unknown-unknown target.

### Done

- ✅ **Phase 2 — common crate** (2026-08-13, commits `1c8d543`..`22a4f3c`):
  9 modules (`text, name, tag, response, hash, time, pow, request, search`),
  TDD red→green, **104 tests green** at `test/unit/common/<module>/tests.rs`
  via `#[path]`. Owner refinements in `document/adjudication.md`: typed
  lowercase `DeleteMode`, unified `DeleteBody`, generic `err()`, panic-free
  `token() -> anyhow::Result`, both-or-neither check, no tracing, RFC3339
  offset formatter (#19), `SearchHit` single-sourced (#10).
- ✅ **Phase 3 slice 1 — authentication/session** (2026-08-13, commit
  `6c063b0`): `GET /challenge/read`, `POST /email/read?intent=...`,
  `POST /user/create`, `GET /session/read`, `POST /session/delete` across the
  four layers; **back 59 tests green**. Back module tree = ADR-0001; `intent`
  contract = ADR-0002 (#26 closed); glossary = `document/context.md`.
  `change_email`/`deregister` branches stub 400 until slice 2.
  - §8 gate: equal-or-better on all five axes — `TokenCache<E>` replaces the
    legacy six-file token duplication, `EmailSender` seam, explicit `intent`,
    corrected error semantics (AC4/AC5). Probe finding: moka's eviction
    listener is housekeeper-driven (eventually consistent), same as legacy.
- ✅ **Phase 3 slice 2 — user domain** (2026-08-13, commits `635a53e`..`acca62a`):
  `GET /user/{id}/read`, `GET /user/read`, `POST /user/{id}/update`,
  `POST /user/{id}/delete`, and the `change_email`/`deregister` branches of
  `POST /email/read` (stubs removed). **back 117 tests green** (common 104).
  - #15: symmetric `email_hash` defaults (false everywhere); self-read errors
    surfaced, not swallowed. #27: idempotent deregister confirm (200 when the
    token is missing and the user is already gone). #25: `has_next = page <
    total_pages`.
  - New repository modules `transfer.rs` (recycler selection + account-asset
    transfer) and `delete.rs` (hard delete); `role.rs` gained
    `user_holds_permission` (interim admin-console gate until Cedar lands in
    slice 6); `cache.rs` gained `EmailUpdateTokenEntry`/`DeregisterTokenEntry`
    and an atomic `consume_if` (moka `and_compute_with`, key-serialized).
  - Deferred to later slices (no stubs): search-index re-sync after name
    update / deregister (slice 3), article/version/comment cascade + PDF
    cleanup on hard delete (slices 3/4/5), recycler least-loaded selection is
    only exercised once articles exist (slice 3), pagination constants move
    to toml at slice 7.
  - §8.3 gate: equal-or-better. Library facts read on disk: agdb `remove`
    cascades node edges (`remove_query.rs`); moka `and_compute_with` is
    key-serialized (`entry_selector.rs`) and `Entry::value()` returns a clone.
    `actor_id` is passed directly (no redundant re-authentication);
    `EmailMismatch` replaces the legacy opaque `bool`; one
    `send_confirmation_email` helper replaces three near-identical blocks.
- ✅ **Phase 3 slice 3 — article + version** (2026-08-13, commits `8de3490`
  archive + `6747cad` rewrite): the owner ordered a clean rewrite of slices 1-3
  under the new CRUD-only vocabulary. The backend was rebuilt from the empty
  skeleton via TDD (sub-agents). **back 180 tests green** (common 104; +2 §8.3
  probe tests),
  `cargo check` zero warnings. The `_`-prefixed archive files were removed on
  2026-08-14 (`commit` below) — the old implementation remains recoverable from
  git (`8de3490^`); it was never referenced by the active tree.
  - Covers all 19 slice 1-3 routes (challenge/session, user, article/version)
    across the four layers. Adjudication: #6 (version list read-open), #15, #17
    (distinct `ContentHashTaken`), #18 (`latest_version_id` in the article
    list), #20 (no unconditional startup rebuild), #21 (seekstorm count), #23
    (no redundant author lookup), #25, #26, #27. Slice-2 deferred items done:
    search re-sync after rename/deregister (`sync`/`sync_user`/`sync_all`,
    best-effort), article/version/comment cascade + PDF cleanup on hard delete,
    recycler least-loaded selection.
  - Repository interfaces designed fresh per §4.1: typed `ArticleDraft`/
    `ArticleUpdate`/`VersionDraft` inputs, a `SearchIndex` struct
    (`open_or_create`/`sync`/`sync_user`/`sync_all`/`read`/`close`), and fresh query
    names (`owner_of`/`content_hash_owner`/`parent_article_of`/`versions_of`)
    instead of legacy-verbatim names.
  - §8.3 gate: equal-or-better on all five axes, grounded in on-disk library
    source + probe tests (2 added). Correctness: (a) agdb serializes writes —
    `agdb-0.13.2/src/db.rs` documents "only single write operation at any one
    time" and `transaction_mut` commits on `Ok`/rolls back on `Err`; the
    content-hash uniqueness is checked inside the txn (`find_by_index_in_txn`),
    probed by `concurrent_identical_content_hashes_are_serialized_by_the_write_lock`
    (exactly one of two racing identical hashes wins). (b) seekstorm count
    (#21) is single-sourced — `seekstorm-3.3.5/src/index.rs` defines
    `current_doc_count` = "indexed - deleted" and `update_document` = "delete +
    index"; `read` uses `result_count_total`; probed by
    `sync_all_and_incremental_sync_agree_on_document_count` (incremental and full
    rebuild both report 2, then 1 after a delete). (c) PDF boundaries — `%PDF-`
    header + `1.x`/`2.x` version + `%%EOF` footer within the last 1024 bytes,
    ≥10 bytes, ≤`max_pdf_size_bytes` — probed by the `infrastructure/pdf.rs`
    accept/reject matrix. (d) `PdfUpload` RAII — `Received` drops the temp file,
    `Placed` drops the final file unless `keep_final()`, `Kept` persists — probed
    by `upload_places_the_pdf_and_drops_an_unkept_placed_file`. Elegance/
    conciseness: one `infrastructure/pdf.rs` for stream validation + placement +
    RAII; a 6-method `SearchIndex` replaces the legacy search duplication.
    Performance: incremental `sync` reuses `update_document`/`index_document` (no
    full rebuild). New probe finding: seekstorm 3.3.5 does not release RAM on
    `IndexArc` drop — `index.rs` `Close` ("Remove index from RAM") must be called
    manually. Added `SearchIndex::close()` (wired into graceful shutdown, FR-7)
    and closed the index in search tests; this also fixed an OOM once the two
    probe tests pushed the un-closed index set over the memory limit.
  - ⚠️ **Tooling (corrected)**: the file tools (`read_file`/`write_file`/
    `edit_file`) operate on real paths — verified with a sentinel write while
    `_X.rs` existed (the write landed in `X.rs`). The earlier "transparent
    mapping" warning in this file was wrong and is removed.

### Owner decisions (2026-08-13)

- **#26 closed**: `intent` is a query parameter, not a body field.
- **#5 confirmed**: read-open semantics — any authenticated principal may
  read article/version/comment; writes stay owner/role/admin-gated.
- **CRUD-only resource vocabulary (new)**. Backend resources are operated on
  with exactly `create`/`read`/`update`/`delete`; batch reads are `read` with
  pagination params (no `list`). Frontend/wire flow terms (`intent=
  authenticate|change_email|deregister`) must not appear as backend
  identifiers — the backend names the node op (`create_user`,
  `update_user_email`, `delete_user`, `read_session`, `delete_session`).
  Interface = strictest (`<verb>_<resource>` handlers); logic top-level = same
  verbs; below them repository/infrastructure use their own terms. Enforced
  across slice 3 (sweep commit `43e3bff`).
- Common API contracts at Phase 3 call sites: `now_ms() -> Result<u64,
  SystemTimeError>` (propagate with `?`), `uuidv7_timestamp_ms` returns
  `None` for non-v7 ids, `format_rfc3339_with_offset(utc_ms, offset_seconds)`
  accepts whole-minute offsets only (extremes ±23:59; `Z` for UTC).

## Handover (2026-08-14) — current agent

Personnel change: the agent that completed slice 3 (agent D) has been
replaced; a new agent (E) takes over from **slice 4 (comment domain)**.

- ✅ Slice 3 (article + version) is done: TDD rewrite of slices 1-3 under the
  CRUD-only vocabulary (commits `8de3490` archive + `6747cad` rewrite),
  **back 180 tests green** (common 104; +2 §8.3 probe tests), `cargo check`
  zero warnings, working tree clean. The `_`-prefixed archive files were
  removed on 2026-08-14
  (commit `aa39cb6`); the pre-rewrite implementation remains recoverable from
  git (`8de3490^`). The file tools operate on real paths (the old "mapping"
  warning was false and has been removed from this file).
- ✅ **Slice 3 §8.3 evidence gap closed**: the five-axis comparison in Current
  state is now grounded with library-source citations and probe tests (2 added);
  agdb write transactions, seekstorm `sync` vs `sync_all` count consistency
  (#21), PDF stream validation boundaries, and `PdfUpload` RAII cleanup paths
  are all evidence-backed.
- Next: **slice 4 (comment domain)** — create/reply/read/update/delete with
  the `DeleteBody` mode contract (#2), then slice 5 (download/PDF,
  #1/#28/#29), slice 6 (role/authorization, #5/#7/#8/#9), slice 7
  (config/email/infrastructure, #13/#19/#22/#26/#32).
- All prior handover items (a)-(d) and the slice 2 deferred items (search
  re-sync, hard-delete cascade + PDF cleanup, recycler least-loaded selection)
  are ✅ done.
- `thermo-nuclear-code-quality-review` is available to this session; invoke it
  together with the §8.3 gate at the end of each slice; its 1k-line default
  bar is superseded by README §5.3 (512 lines).

## Rules (non-negotiable; operational only)

Read `nail_new/README.md` in full first — it is the constitution and the
single source of rules; nothing is repeated here. Not in the README, still
binding:

- One task = one commit (`git add .` + commit + push) with an English message
  reflecting the actual change; archive before and after each task.
- Kill leftover backend/proxy processes after e2e (they lock the agdb file).
- Keep `document/adjudication.md` current as decisions refine; record new
  contracts as ADRs under `document/`.

## Reference materials

- `nail_new/README.md` — the constitution (read first).
- `document/adjudication.md` — the 32 verdicts to implement.
- `document/context.md` — glossary (domain-modeling output).
- `document/adr/` — 0001 (back module tree), 0002 (`intent` contract).
- `document/reconstruction/features/02-code/PRD.md` — the domain spec (FRs
  tagged [confirmed]/[inferred], acceptance criteria, test plan).
- `document/reconstruction/architecture/INTERFACES.md` — route contracts.
- `document/reconstruction/architecture/DATA-MODEL.md` — entities, enums.
- `document/reconstruction/architecture/ARCHITECTURE.md` — layer mapping.
- `document/reference/code/` — the legacy code (untrusted; probe to verify).
- `nail` (frozen) — git archaeology if a detail is missing.

## What the target is (and is not)

- The target is **PRD requirements + adjudication verdicts + the README
  architecture rules** — the *corrected* behavior, not a copy of the legacy.
- The reconstruction PRDs are an inventory (including the bugs), not a spec;
  when the legacy contradicts a verdict, the verdict wins.
- Anything new found during migration: probe it, then report to the owner —
  never silently preserve a bug or a compromise design.

## Remaining steps

### Phase 3 — backend migration, one domain per slice (in progress)

Per domain: read the PRD slice → write failing tests first (red) → implement
(green) → commit. Order:

1. ✅ Authentication/session — done (see Current state).
2. ✅ User domain — #15 (symmetric email_hash defaults, no swallowed errors),
   #27 (idempotent deregister confirm) — done (see Current state).
3. ✅ Article + version — #6, #16, #17, #18, #20, #21, #23 — done (see Current
   state; rewritten via TDD under the CRUD vocabulary).
4. Comment domain — create/reply/list/update/delete with the `DeleteBody`
   mode contract (#2).
5. Download/PDF — #1 (mint → `.../content/read?token={token}`, single-use
   60s, user-bound), #28 (hash-based filenames only), #29 (mint-JSON
   contract documented).
6. Role/authorization — #5 (visibility deleted; policy 2 rewritten to the
   owner-confirmed read-open semantics), #7 (member_count), #8 (REQUIRED_ROLES
   protected), #9 (duplicate role → 400).
7. Config/email/infrastructure — #13 (dead config fields), #19 (timezone from
   toml via `/config/read`), #22 (multipart read-then-validate), #26 (email
   service with explicit intent), #32 (e2e flag; test tree in Phase 5).

Layering per ARCHITECTURE.md: interface = HTTP + envelope + session-token +
PoW placement; logic = business rules (near-pure); repository = agdb + moka +
seekstorm; infrastructure = axum bootstrap, toml conf, SMTP client, PDF store,
tracing, Cedar engine.

### Phase 4 — frontend migration (after the backend API is stable)

- Layering per README §4.2 (router → page → request → infrastructure);
  Leptos CSR, no CSS (README §10); runtime config from `/config/read` with
  compile-time fallback.
- Items: #14 (all UI strings English), #24 (page size from config), #25
  (has_next = page < total_pages), #3 (no per-comment pre-check), #12 (drop
  `/private/email/check` link), #4 (delete `pow-worker.js`).
- Gate: `cargo check --target wasm32-unknown-unknown` on `nail_front`.

### Phase 5 — tests + end-to-end + cleanup

- Rebuild the test tree per README §12: unit families at the four backend
  seams + common.
- e2e strategy (#32): decide with the owner (the old design: real process
  stack back + pingap + chromium with an in-process SMTP sink).
- Final batch dead-code cleanup (README §5.4): remove every dead/unused item
  and the `#![allow(dead_code)]` crate-root attributes in one dedicated pass,
  then re-run the full `cargo test` and `cargo check` with zero warnings.
- Gate: full `cargo test` suites; e2e serial (`--test-threads=1`) if ports
  are shared.

## Skills — mandatory usage

Skills are MANDATORY, not optional: before every covered task, invoke the
matching skill via the `skill` tool and follow it — skipping one is a process
violation. The README is the constitution and outranks any skill. Do not
grill routine work.

| Skill | MUST invoke before | Notes for this repo |
| --- | --- | --- |
| setup-matt-pocock-skills | the first session, once | already done |
| tdd | every implementation slice | agree seams with the owner first; one failing test → minimal implementation; tests through public interfaces; the PRDs carry `--tdd` |
| codebase-design | every module-tree / boundary design | README §4 fixes direction only; this decides depth/shape/seams |
| diagnosing-bugs | behavior contradicts expectations | reproduce → isolate → root cause → fix; report findings to the owner |
| domain-modeling | start of Phase 3 + any new term | glossary already at `document/context.md` |
| grilling | an open/risky owner decision | one question at a time, recommend an answer each round |
| grill-with-docs | an open decision that lands as ADRs + glossary | use when the outcome must be documented |
| handoff | end of every slice, phase, and session | update `document/handoff.md`; keep adjudication current |
| to-spec | when the owner wants a formal spec | optional — PRDs + adjudication already act as specs |
| improve-codebase-architecture | periodic architecture health check | not needed during the build |
| thermo-nuclear-code-quality-review | code-quality review of each completed slice/phase (with the §8.3 gate) or on owner request | aggressive on structure, abstractions, spaghetti, file size; project file bar is 512 lines (README §5.3), stricter than the skill's 1k default |
| create-skill | only if the owner asks | — |
