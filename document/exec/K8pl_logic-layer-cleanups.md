# Exec — Task VIa: backend LOGIC layer low-risk cleanups

**Task**: VIa (Task V follow-ups + logic-layer dedup). **Owner**: L9sPvT.
**Scope root**: `code/back/src/logic/**`, `code/back/src/repository/user.rs`,
`code/back/src/repository/graph.rs` (helper deletion only),
`test/unit/back/logic/**`, `test/unit/back/repository/user.rs`,
`test/unit/back/harness.rs` (additive probe module line only).

## 1. Requirement

Behavior-preserving dedup, five stages, back crate only:

- **A** — `logic/pagination.rs` owns the offset math, the usize guards, `skip/take`,
  and `has_next`; all 7 hand-inlined pagination blocks (user.rs, comment.rs x2,
  version.rs, search.rs, role.rs, tag.rs) call it.
- **B** — one token-lifecycle helper owns normalize → hash → error mapping;
  ~11 repeated `normalize_token`/`token_key`/`map_err(format!(...))` blocks across
  `logic/{user,email,download,session}.rs` replaced.
- **C** — the duplicated "identical PDF already exists" rejection becomes one
  helper shared by `logic/article.rs` and `logic/version.rs`.
- **D** — delete empty placeholder dirs `logic/{authenticate,challenge,email,error,
  pow,session}/` (`.gitkeep` only) plus redundant `logic/.gitkeep` (dir is not
  empty); no other dead code exists in `logic/**` (evidence: zero clippy warnings,
  zero `#[allow(dead_code)]` in `code/back/src`).
- **E** — migrate `repository/user.rs` off the 3 remaining `_sync` helpers
  (`resolve_node_id_sync`, `find_by_index_sync`, `read_rows_sync`); delete the
  helpers from `graph.rs` (grep: only user.rs references them).

Acceptance: no message/status/flow change; 543/543 back tests; fmt + clippy
zero-warning gates per slice; one commit per slice.

## 2. Scope

In: files listed above; `test/unit/back/harness.rs` one additive `#[path]` line
for probe 003 (never touch other agents' lines).

Out: `interface/**`, `infrastructure/**` (STOP + report if needed),
`code/front/**`, `code/common/**`, other repository files, other agents' docs.
`clamp_page_limit` stays byte-identical. No behavior/message/policy changes.

## 3. Design decisions

### Stage A — two helpers, one home (pagination.rs)

- `pub fn page_offset(page: u64, limit: u64) -> u64` — owns
  `page.saturating_sub(1).saturating_mul(limit)`. Used by the 3 repository-driven
  pagers (comment.rs x2, version.rs) and search.rs's SeekStorm offset.
- `pub fn paginate<T>(items: Vec<T>, page: u64, limit: u64) -> (Vec<T>, bool)`
  — owns `page_offset`, the `usize::try_from(...).unwrap_or(usize::MAX)` guards
  (huge page → empty page, never panic), `skip/take`, and
  `has_next = page < total.div_ceil(limit)`.
- Callers needing `total` (user.rs, role.rs, tag.rs) compute
  `let total = items.len() as u64;` before consuming — identical value.
- search.rs: SeekStorm offset via `page_offset`; tree slicing via `paginate`.
  Its legacy `has_next = len as u64 > offset + limit` is mathematically identical
  to `page < total.div_ceil(limit)` for limit ≥ 1 (all callers clamp via
  `clamp_page_limit`; interface/user,role,tag,comment,version + search all clamp —
  verified by grep). Probe 003 brute-forces this equivalence.
- `paginate` consumes the `Vec` (user/role/search already own; tag.rs switches
  `.iter()` collect to owned — element order unchanged, hashing of refs not needed).

### Stage B — two helpers in logic/session.rs (home of `normalize_token`)

```rust
pub fn token_key(token: &str) -> Result<String, LogicError>            // hash canonical token
pub fn hash_token(raw: &str, invalid: LogicError) -> Result<String, LogicError>
// = normalize_token(raw).ok_or(invalid) + token_key
```

- Both funnel the hash error to ONE internal message: `"failed to hash token: {error}"`
  — consolidating 5 today-different strings (email/delete/session/create-user/
  download token). See Questions Q1. All are `LogicError::Internal` (500) on a
  path that cannot trigger (source: common/src/hash.rs:16 — ascon
  `try_new_customized` fails only on invalid customization bytes; salt is the
  fixed literal `b"token-hash"`).
- Per-flow user-facing normalize-failure messages (bad_request/unauthorized)
  are preserved exactly: "invalid or expired token", "invalid delete token",
  "invalid session", "invalid old/new email token", "invalid or expired download
  token" — they carry real semantic differences, passed as the `invalid` arg.
- `invalid` is built eagerly at call sites (original was lazy `ok_or_else`);
  delta = one small `String` alloc on success paths. Negligible; noted for review.
- No `store_confirmation_token`: the 3 confirmation caches (create_user /
  delete_user / email_update) hold different entry structs with different
  reverse-key semantics; hash is the only shared mechanic, so only it is extracted.
- `email.rs update_user_email` keeps its early `normalize_token` calls (canonical
  strings are compared against `pow.payload` before hashing); only the hashing
  moves to `token_key`.
- session.rs keeps its `repository::cache::{SessionTokenEntry, token_key}`
  import; new names (`hash_token`, `token_key`... conflict!) — see note below.

  NOTE: `logic::session::token_key` vs `repository::cache::token_key` name
  collision in session.rs → local helper is named `hash_canonical_token` instead.
  Call sites import `hash_token` + `hash_canonical_token` from logic::session and
  drop their `repository::cache::token_key` imports (user.rs, email.rs,
  download.rs). Tests keep using `repository::cache::token_key` (unchanged).

### Stage C — one dedup helper in logic/version.rs

`pub(crate) async fn reject_duplicate_content_hash(state, hash) -> Result<(), LogicError>`
moved from article.rs (identical logic already lives there); version.rs's inline
copy replaced. Exact message preserved: `"identical PDF already exists (version {n})"`
with `read_version(...).map(|e| e.version_number).unwrap_or_default()` fallback.
Home: version.rs — article.rs already imports place_uploaded_pdf/validate_* from it.

### Stage D — deletions

`git rm` the 6 dirs + root `logic/.gitkeep` (verified `.gitkeep`-only via
`git ls-files` + `ls -la`). No other dead scaffolding in `logic/**`
(grep `allow(dead_code)` = none in `code/back/src`; clippy zero-warning gate
already proves no unused pub items in this binary crate).

### Stage E — user.rs onto GraphQuery

Mirror tag.rs/delete.rs pattern: `find_by_index`, `resolve_node_id`,
`read_node` (read_node == `read_rows(...).into_iter().next()` by definition,
graph.rs:161-168 — source evidence). Read and write guards both implement
GraphQuery. `read_user`'s `DbError::query(NotFound, "user row missing")`
construction preserved. Then delete the 3 `_sync` helpers (only user.rs uses
them — repo-scope grep). `has_soft_deleted_flag(&guard, ...)` already takes
`&impl GraphQuery` (delete.rs:78) — untouched.

## 4. Slice breakdown

| # | Stage | Red | Green | Exit test |
|---|---|---|---|---|
| 1 | A | probe 003 + paginate/page_offset unit tests (compile-fail) | helpers + 7 sites rewired | fmt, clippy, logic_ + http_ groups |
| 2 | B | hash_token/hash_canonical_token unit tests (compile-fail) | helpers + ~11 sites rewired | fmt, clippy, logic_ + http_ groups |
| 3 | C | reject_duplicate_content_hash unit test (compile-fail) | helper moved, both callers wired | fmt, clippy, logic_ + http_ groups |
| 4 | E | N/A — internal rewrite, no new behavior; existing 12 repository_user tests are the proof | user.rs migrated, `_sync` helpers deleted | fmt, clippy, repository_ group + cargo check |
| 5 | D | N/A — file deletion only | dirs gone, `git ls-files` clean | fmt, clippy, cargo check |

Final gate: full 543-test split + fmt + clippy.

## 5. Open unknowns

- U1 (probe 003): `paginate` has_next formula vs search.rs legacy formula
  equivalence — brute-force property test.
- U2 (source): `read_node` ≡ `read_rows().next()` — graph.rs:161-168, no probe.
- U3 (source): hash error path unreachable with fixed salt — common/src/hash.rs:16.
- U4 (grep): `_sync` helpers referenced only by user.rs — confirmed.

## 6. Verification plan

- Correctness: per-slice module groups + full final gate (543).
- Behavior change: none by construction; message diffs only the 5→1 hash-error
  consolidation (Q1); probe equivalence evidence.
- Time/space: paginate identical ops; eager `invalid` alloc noted (Q2).
- Performance: no measurable delta (same queries, same skip/take).

## 7. Risks

- Concurrent agents on harness.rs/main tree: only additive probe line; stage
  only own paths; re-read files before each slice.
- Probe numbering collision: 003 free (001/002 exist).
- OOM: run tests split per module, serially; check `uptime` before every build.
- Rollback: each slice is one commit; revert that commit restores prior state.

## 8. Constraints

- Back crate only; never hand-edit Cargo.lock; never touch target/dist/data/log.
- `clamp_page_limit` unchanged; no new panics; English only; no comments
  restating code; stage only own paths; one commit per slice.
- No message/status/flow changes without orchestrator approval.

## 9. Questions

- Q1: Consolidate the 5 per-flow internal hash-failure strings
  ("failed to hash {email|delete|session|create-user|download} token") to one
  `"failed to hash token"`? (Unreachable in practice; all are 500s.) Reject →
  parameterize the helper with the per-flow noun instead.
- Q2: Accept eager construction of the `invalid` LogicError at call sites
  (one small String alloc on success paths) vs. closure-based laziness?
- Q3: Delete root `logic/.gitkeep` along with the 6 subdirs (root dir is not
  empty, marker is redundant)?

## Change log

- 2026-08-19 S1: Probe 003 FINDING — legacy has_next formulas disagree at
  page==0: user/role/tag used `page < total.div_ceil(limit)`, search.rs used
  `len > offset + limit` (diverge when 0 < total <= limit, page 0). Page 0 is
  unreachable in production (`clamp_page_limit` clamps to >= 1; only direct
  logic-layer test calls hit it). Decision (recorded): helper unifies on
  div_ceil; search.rs page-0 behavior now matches user/role/tag. No existing
  test asserted the old value. Permanent tests pin the reachable domain
  (page >= 1); probe 003 deleted per orchestrator instruction (equivalence
  covered by permanent tests); harness.rs line removed. Slices: docs commit +
  Stage A commit.