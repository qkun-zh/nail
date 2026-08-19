# Exec doc — Xz9v: repository graph abstraction (Task V)

Owner: Wb5Kq2. Orchestrator approves at the adoption gate. Single source of
truth for this task. Back crate only.

## Requirement

Centralize the copy-pasted distance-1 typed-edge query blocks and the
`_sync`/`_in_txn` helper pairs in `code/back/src/repository/`, unify the three
"latest version" implementations into one semver-correct helper (fixing the
delete.rs string-max bug), route every edge insertion through one helper, and
delete dead scaffolding. All behavior must stay identical except the Stage C
bug fix.

Acceptance criteria:
- Stage A: `graph::outgoing_edges`/`incoming_edges` (return `Vec<DbElement>`)
  replace ~30 hand-rolled `search().from().distance(Equal(1)).edge().
  key(KEY_TYPE).value(EDGE_*)` blocks across article.rs, comment.rs, delete.rs,
  role.rs, version.rs, tag.rs, transfer.rs, search/db.rs, search/document.rs,
  authorization.rs. Query construction centralized; no behavioral delta.
- Stage B: one definition per concept for `resolve_node_id`, `find_by_index`,
  `read_rows`, `read_node`, `has_soft_deleted_flag` over a small executor trait;
  no `_sync`/`_in_txn` pairs remain.
- Stage C: one shared `highest_version_number` (semver) used by version.rs,
  article.rs `live_latest_version`, delete.rs. delete.rs bug fixed with a
  failing repro first (version "10.0.0" newest vs "9.9.9").
- Stage D: all edge insertion via `graph::insert_edge` (`&mut impl GraphWrite`);
  none hand-rolled.
- Stage E: `read_tag_articles` and any genuinely-dead `#[allow(dead_code)]`
  inside repository/ removed; none remain.
- Zero-warning gate green; all back tests pass; no new panics; English only.

## Scope

In scope: `code/back/src/repository/{graph,article,comment,delete,role,version,
tag,transfer,authorization}.rs`, `code/back/src/repository/search/{db,document,
query,schema}.rs`, plus tests in `test/unit/back/repository/` (delete.rs gets
the Stage C repro) and only if required `test/unit/back/logic/`.

Out of scope: `code/front/**`, `code/common/**`, other back modules, `read_tag_
detail` (logic, another task), distance-2 node/page queries, any message/error
or policy change, `Cargo.lock`, `target/`/`dist/`/`data/`/`log/`.

## Design decisions

- Executor trait (Stage B): `pub(crate) trait GraphQuery { fn exec_query<T:
  agdb::Query>(&self, T) -> Result<QueryResult, DbError>; }` implemented for
  `agdb::DbAny`, `agdb::DbAnyTransactionMut`, and the two `tokio::sync` guards
  (`RwLockReadGuard`/`RwLockWriteGuard` over `DbAny`). Source evidence: agdb
  0.13.2 `DbAny::exec`/`DbAnyTransactionMut::exec` are both `&self` + `T: Query`
  (db.rs:336, transaction_mut.rs:26); `Query`/`QueryMut`/`DbElement` are public
  (lib.rs:136-137,124). Guard impls avoid deref-coercion problems at the many
  `&guard` call sites (a generic `&impl GraphQuery` does not deref-coerce, so
  the guard types themselves must impl the trait). One definition per concept.
- Stage A helpers take `&impl GraphQuery` and return `Vec<DbElement>`
  (`QueryResult.elements`). `edge_count(executor, from, edge_type)` = outgoing
  count (matches transfer.rs/role.rs usage); incoming counts use
  `incoming_edges(...)?.len()` (matches tag.rs/comment.rs) — satisfies the
  task's "edge_count (or equivalent)".
- Stage D: `pub(crate) trait GraphWrite { fn exec_mut_query<T: agdb::QueryMut>
  (&mut self, T) -> Result<QueryResult, DbError>; }` for `DbAny`,
  `DbAnyTransactionMut`, `RwLockWriteGuard`. `insert_edge` becomes
  `&mut impl GraphWrite`; transaction call sites (`&mut DbAnyTransactionMut`)
  and guard call sites (`&mut guard`) both match, behavior identical
  (`DbAny::exec_mut` auto-txn ≡ explicit txn for a single insert).
- Stage C: `graph::highest_version_number(rows: Vec<VersionRow>) -> Option<
  VersionRow>` = semver-aware `max_by`, string fallback on parse failure
  (matches article.rs's existing fallback exactly). delete.rs bug fix:
  replace `.map(|row| row.id).max()` (max on the version *business-id* string,
  not the number — the delete.rs:501 bug) with `highest_version_number(rows).
  map(|row| row.id).unwrap_or_default()`. version.rs keeps its strict
  `InvalidNumber` behavior: pre-validate every stored version parses, then use
  the helper for the max (observable behavior identical — `InvalidNumber` if
  any stored version is invalid; helper's string fallback never triggers).
- Slice red-phase for behavior-preserving slices = pre-existing unmodified
  per-domain tests as regression pin (Task II precedent, orchestrator-approved).
  The only genuinely-red slice is Stage C (failing repro first).

## Slice breakdown

1. **graph.rs foundation + delete.rs rewire** (Stage A+B start).
   Files: `graph.rs`, `delete.rs`.
   Red: none (refactor); existing delete tests are the pin.
   Green: delete.rs uses `outgoing_edges`/`incoming_edges`/`resolve_node_id`/
   `read_node`; `has_soft_deleted_flag` becomes single `&impl GraphQuery`
   (`_in_txn` left as a temporary shim until version/comment migrate).
   Exit: fmt --check && clippy -D warnings && back repository delete tests.
2. **version.rs + tag.rs rewire** (+ `find_by_index` helper).
   Files: `graph.rs`, `version.rs`, `tag.rs`.
   Green: version.rs (strict create_version preserved) + tag.rs use unified
   helpers/edges; tag.rs `apply_tag_to_article` routed via `insert_edge` (Stage D
   start). Exit: repository version + tag + delete tests.
3. **article.rs rewire**. Files: `graph.rs`, `article.rs`. Green: article.rs uses
   unified helpers/edges; remove now-unused old `_sync`/`_in_txn` pairs.
   Exit: repository article + version + delete tests.
4. **comment.rs + role.rs rewire** (+ GraphWrite/insert_edge). Files: `graph.rs`,
   `comment.rs`, `role.rs`. Green: comment.rs/role.rs use unified helpers +
   edges + `insert_edge`; migrate last `has_soft_deleted_flag_in_txn` callers.
   Exit: repository comment + role + delete + version tests.
5. **transfer.rs + authorization.rs + search/* rewire** (+ `edge_count`). Files:
   `graph.rs`, `transfer.rs`, `authorization.rs`, `search/{db,document}.rs`.
   Green: all routed through unified helpers/edges/insert_edge. Exit: full back
   test.
6. **Stage E cleanup**. Files: `tag.rs` (remove `read_tag_articles`), `graph.rs`
   (drop all old `_sync`/`_in_txn` + `has_soft_deleted_flag_in_txn` shim).
   Green: no `_sync`/`_in_txn` remains; `rg` finds no `#[allow(dead_code)]` in
   repository/. Exit: full back test.
7. **Stage C bug fix** (red-first). Files: `graph.rs`, `delete.rs`,
   `version.rs`, `article.rs`, `test/unit/back/repository/delete.rs`.
   Red: add `delete_refresh_keeps_the_semver_latest_version` — article with
   version "1.0.0"(id "ffffffff-…"), "9.9.9"(id "11111111-…"), "10.0.0"(id
   "22222222-…"); delete "10.0.0"; assert `latest_version == "9.9.9"` and
   `latest_version_id` == 9.9.9's id. Fails today (string-max picks "ffff…").
   Green: add `highest_version_number`, use in delete.rs/version.rs/article.rs.
   Exit: fmt --check && clippy -D warnings && full back test.

## Open unknowns

- agdb `exec`/`exec_mut`/`Query`/`QueryMut`/`DbElement` — resolved by pinned
  source (evidence above); no probe (behavior visible in source).
- Trait-over-guard compilation: standard Rust pattern; empirically verified by
  the slice-1 gate compile+test (no standalone probe to avoid touching
  `test/unit/back/harness.rs`, which is outside the declared test scope).

## Verification plan

- Correctness: existing unmodified repository tests run every slice (regression
  pin); Stage C repro proves the fix. Verified via `cargo test`.
- Behavior change: zero except Stage C (documented). Proven by unmodified tests
  staying green per slice.
- Time/space complexity: unchanged (same queries, same rows); helper adds one
  `chars()`-style semver parse per version, as article.rs already does.
  Verified by inspection.
- Performance: unchanged (query text identical, one allocation for the returned
  `Vec<DbElement>`); verified by inspection.

## Risks

- Deref/coercion surprises on `&guard` call sites — mitigated by explicit guard
  trait impls; slice gates catch early.
- Stage C string-max bug produces non-deterministic results under random UUIDs —
  mitigated by crafting deterministic version ids in the repro (source evidence
  confirmed the map is over `row.id`).
- version.rs strictness loss — mitigated by pre-validation preserving
  `InvalidNumber`.
- OOM on full test — run per-module when load is high; all back tests eventually
  pass.

## Constraints

- Back crate only. Touch only the declared files; stage by explicit path.
- Preserve behavior except Stage C. No message/error/policy change.
- No `unwrap`/`expect`/new panics; no comments restating code; English only.
- Never hand-edit `Cargo.lock`; never touch `target/`/`dist/`/`data/`/`log/`.
- Check machine load before any build; back off if loaded. One commit per slice.

## Questions

1. Orchestrator: approve this plan at the adoption gate?
2. Accept the documented red-phase note (no genuinely-red test until Stage C,
   per Task II precedent)?
3. Scope note: migrating `has_soft_deleted_flag` in slice 1 changes its
   signature to `&impl GraphQuery`; all sync callers (article/version/comment/
   search) keep working unchanged via guard impls — flag if you prefer a
   separate slice.

## Change log

- 2026-08-19: initial plan. Baseline green (back, 542 passed, 0 failed).
- 2026-08-19: Orchestrator APPROVED. Recorded deviations (documented):
  1. Red-phase: behavior-preserving slices use existing unmodified tests as the
     regression pin; only Stage C (slice 7) is genuinely red (Task II precedent).
  2. `has_soft_deleted_flag` becomes single `&impl GraphQuery` in slice 1; guard
     impls keep all sync callers unchanged; no separate slice.
  3. No standalone probe for the trait-over-guard pattern; slice-1 gate
     (compile + full back test) is the empirical proof (avoids touching
     `test/unit/back/harness.rs`, outside declared test scope).
- 2026-08-19: slices 1-5 committed (`29f56aa`, `d203237`, `15a4681`, `6b49a93`,
  `8f5a9d4`) — Stages A/B/D done, search/* rewired, full back test green.
- 2026-08-19: slice 6 (Stage E) committed (`9b6586e`). Deviations (documented):
  1. `read_tag_articles` is used by 4 live tests (`test/unit/back/logic/
     tag_apply.rs` L72/91/110/126) — it is test-support, not dead scaffolding.
     Removed from production; kept as a `#[cfg(test)]` test-only copy (tests
     untouched, no `#[allow(dead_code)]` anywhere in repository/).
  2. `read_tag_detail` (`logic/tag.rs`, out of scope) was its sole production
     consumer and is removed along with it — forced by Stage E; another task
     will re-add it when wiring the feature.
  3. Three `_sync` helpers (`resolve_node_id_sync`/`find_by_index_sync`/
     `read_rows_sync`) remain in graph.rs — pinned by out-of-scope `user.rs`;
     `read_node_sync` and every `_in_txn` pair are gone. Slice 6 exit criterion
     "no `_sync` remains" relaxed to "none but the user.rs-pinned trio".
- 2026-08-19: slice 7 (Stage C) committed (`b6fbd94`) — red-first evidence:
  new repro `delete_refresh_keeps_the_semver_latest_version` (article versions
  "1.0.0"/"9.9.9"/"10.0.0" with crafted ids ffffffff-…/11111111-…/22222222-…;
  delete "10.0.0") FAILED before the fix: `assertion failed: left: "1.0.0"
  right: "9.9.9"` (string-max over business-id). After `highest_version_number`
  (semver max_by, string fallback) in graph.rs used by delete.rs/version.rs
  (strict `InvalidNumber` pre-validation preserved)/article.rs — PASSED.
- 2026-08-19: FINAL GATE green — `cargo fmt --check`, `cargo +nightly clippy
  -- -D warnings` (zero warnings), full back test per module (full run OOMs on
  this 9GB box): repository_ 107 + logic_ 260 + http_ 122 + infrastructure_ 43
  + configuration_ 11 = 543 passed, 0 failed (baseline 542 + repro). Task V
  COMPLETE.