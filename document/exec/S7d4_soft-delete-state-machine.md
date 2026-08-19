# Soft-Delete State Machine — consolidate scattered soft-delete logic

**Owner**: qkun-session
**Status**: Planning — exec doc written, awaiting adoption

## Requirement

Refactor the soft-delete subsystem from its current scattered, implicit form
into one deep, explicit module. Behaviour is **unchanged** in every case; this
is a pure consolidation/deepening task (no bug fix, no permission change, no
message change). The existing cascade counter semantics — a node is hidden iff
its `soft_deleted` count is > 0, where hiding/restoring a subtree root
increments/decrements the count across the whole subtree — is the correctness
kernel and **must be preserved exactly**.

### Acceptance Criteria

1. A single deep module `repository/soft_delete.rs` owns the soft-delete
   lifecycle: `KEY_SOFT_DELETED`, the visibility predicate, the cascade
   `soft_delete`/`undelete`, and the in-transaction variant. The
   repository layer's three
   delete semantics live in three precisely named modules: `soft_delete.rs`,
   `hard_delete.rs` (renamed from `delete.rs`), and `transfer.rs` (unchanged).
   `hard_delete.rs` contains no soft-delete code.
2. Outside `soft_delete.rs`, no code reads or writes the `soft_deleted` key
   directly, and no code calls `has_soft_deleted_flag*` / `adjust_*` /
   `soft_delete_*` / `clear_soft_deleted_flag` / `is_soft_deleted` /
   `undelete_soft_user`. All such call sites route through the module's
   interface.
3. The logic layer's four near-identical `delete_X`(Soft) / `undelete_soft_X`
   shapes collapse onto shared helpers; each entity keeps only its
   entity-specific authorization call and its sync hook.
4. Authorization (`authorize.rs`) no longer queries soft-delete state itself;
   it calls the module's visibility predicate.
5. Behaviour identical: same visibility results, same "already soft-deleted" /
   "not soft-deleted" errors, same cascade, same query-time filtering, same
   search index behaviour. Proven by the existing suite.
6. Zero new panics/unwraps; no comments restating code; English only.

## Scope

### In-scope

- `code/back/src/repository/delete.rs` — split + rename to `hard_delete.rs`:
  soft-delete logic moves to the new module; the renamed file keeps only hard
  delete.
- `code/back/src/repository/soft_delete.rs` — new deep module (the state
  machine).
- Call-site conversion in:
  `repository/{article,version,comment,user}.rs`, `repository/search/{db,document}.rs`,
  `logic/{article,version,comment,user}.rs`, `logic/authorize.rs`.
- `test/unit/back/...` — only if a test imports the removed names; no behaviour
  changes expected.

### Out-of-scope

- Cedar schema/policy, permission names, error messages.
- Hard-delete behaviour (`delete_user/article/comment/version`, PDF removal,
  `refresh_latest_version`).
- Transfer behaviour, search ranking, pagination.
- Frontend.
- Other agents' files (harness.rs, probe_*.rs, review-logic-findings.md).

## Design Decisions

### Repository layer: three precise modules

The three delete semantics are cleanly separated at the repository layer, each
in its own module with a precise name (no generic `delete`):

| Semantic | Module | Contents |
|---|---|---|
| Hard delete | `repository/hard_delete.rs` (renamed from `delete.rs`) | `DeleteOutcome`, `delete_user/article/comment/version`, `delete_*_in_txn` cascades, `refresh_latest_version_in_txn` |
| Soft delete | `repository/soft_delete.rs` (new) | `KEY_SOFT_DELETED`, `is_hidden`/`is_hidden_in_txn`, `hide`/`restore`, cascade internals |
| Transfer | `repository/transfer.rs` (unchanged) | `transfer_account_assets`, `transfer_article`, `transfer_comment` |

`hard_delete.rs` and `soft_delete.rs` both live at the repository layer; the
logic layer keeps its `delete_X(mode)` dispatch and the `DeleteMode` enum
unchanged (out of scope).

### Module seam: `repository/soft_delete.rs`

The seam sits at the repository layer because the state machine is a storage
concern (agdb nodes, the `soft_deleted` counter). The interface is small and
deep — three operations plus a predicate:

```rust
// node visibility predicate — is this node in the soft-deleted state?
pub fn is_soft_deleted(guard: &DbAny, id: DbId) -> Result<bool, DbError>;
pub(crate) fn is_soft_deleted_in_txn(tx: &DbAnyTransactionMut, id: DbId) -> Result<bool, DbError>;

// state transitions (cascade across the subtree, delta = +1 / -1)
pub async fn soft_delete(db: &DbHandle, entity_kind: &str, business_id: &str) -> Result<(), DbError>;
pub async fn undelete(db: &DbHandle, entity_kind: &str, business_id: &str) -> Result<(), DbError>;

// query-time filter constant (used with QueryBuilder::not().keys(..))
pub const KEY_SOFT_DELETED: &str = "soft_deleted";
```

Naming follows the state vocabulary: the state is *soft-deleted*, so the
predicate is `is_soft_deleted` and the transitions are `soft_delete` (+1) and
`undelete` (-1, the CRUD verb already used by `undelete_soft_*`). No generic
`hidden`/`hide`/`restore` names leak an ambiguous "hidden by what?" question —
everything is qualified by soft-delete.

- `soft_delete` replaces `soft_delete_article/version/comment/user` (all map to the same
  `adjust_soft_delete_count(.., +1)`).
- `undelete` replaces `clear_soft_deleted_flag` and `undelete_soft_user` (both map
  to `adjust_soft_delete_count(.., -1)`). Its no-op-on-missing behaviour is kept.
- The internal cascade (`adjust_*_subtree_in_txn`, `adjust_user_subtree`,
  `adjust_article_subtree`, `adjust_version_subtree`, `adjust_comment_tree`,
  `adjust_node_soft_delete_count_in_txn`, `soft_delete_count_in_txn`) moves
  verbatim into this module — it is the implementation behind the deep
  interface, not part of the interface.
- `is_soft_deleted` / `is_soft_deleted_in_txn` replace `has_soft_deleted_flag` /
  `has_soft_deleted_flag_in_txn`.

### Query-time filtering stays in place

`.not().keys(KEY_SOFT_DELETED)` (in `live_latest_version`,
`incoming_comment_ids_page`, `versions_of`, search db/document) is a
**performance-necessary** batch filter — it excludes hidden nodes inside agdb,
avoiding per-node reads. It must not be replaced by a Rust-side `is_soft_deleted`
loop (that would be an N+1 regression). It stays, but the string constant is
imported from `soft_delete::KEY_SOFT_DELETED` so the key is owned in one place.

### Logic-layer consolidation

The four entities share the same soft-delete/undelete skeleton (authorize →
check current state → transition → sync). Only the authorization action, the
business-id, and the sync hook differ. The generic shape is moved into a small
logic helper (e.g. in a new `logic/soft_delete.rs` or as free functions), so
each entity's `delete_X(Soft)` / `undelete_soft_X` shrinks to: authorize,
call the helper, run the entity's sync hook. The "already soft-deleted" /
"not soft-deleted" guard and the `is_soft_deleted` pre-check fold into the
helper (state checked atomically within the transition, preserving exact
behaviour — see Risk note below).

Authorisation stays in the logic layer per entity (repository must not know
Cedar). The helper takes the authorisation result as a precondition.

### `hard_delete.rs` after the split

The renamed `hard_delete.rs` keeps: `DeleteOutcome`,
`delete_user/article/comment/version`, the `delete_*_in_txn` cascades,
`refresh_latest_version_in_txn`. Drops all soft-delete items. Renames none of
the retained hard-delete API (out of scope).

## Slice Breakdown

Command template: `cd /home/qkun/nail/code/back && env
CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=cranelift
cargo +nightly test -- --test-threads=1 <filter>`. Check machine load before
every build (workflow §contention).

### Slice 1 — Create `soft_delete.rs`, move mechanism + key

- **Goal**: new module owns KEY_SOFT_DELETED, predicates, cascade;
  `delete.rs` sheds all soft-delete code and is renamed `hard_delete.rs`.
- **Files**: `repository/soft_delete.rs` (new), `repository/delete.rs`
  (→ `repository/hard_delete.rs`), `repository/schema.rs` — remove
  `KEY_SOFT_DELETED` (moved to the new module; its only users —
  `hard_delete.rs` and the query-time filters — switch to the new module's
  constant).
- **Red**: none (pure move; behaviour pinned by existing suite — see Q2).
- **Green**: soft-delete code moved verbatim; `hard_delete.rs` retains only
  hard delete.
- **Exit test**: `logic_soft_delete_visibility` + `logic_delete_verify` +
  `logic_authorize` + `http_content` green + clippy + fmt.

### Slice 2 — Convert repository read/write call sites

- **Goal**: replace 11 `has_soft_deleted_flag*` calls and the scattered key
  literals across repository modules with the new interface.
- **Files**: `repository/{article,version,comment,user}.rs`,
  `repository/search/{db,document}.rs`.
- **Green**: `has_soft_deleted_flag` → `soft_delete::is_hidden`;
  `has_soft_deleted_flag_in_txn` → `soft_delete::is_hidden_in_txn`;
  `.not().keys("soft_deleted")` → `.not().keys(soft_delete::KEY_SOFT_DELETED)`.
- **Exit test**: `logic_search` + `repository_article` + `repository_version`
  + `repository_comment` + `repository_user` + `repository_delete` + `logic_tag_apply`
  green + clippy + fmt.

### Slice 3 — Convert logic delete/undelete + authorize

- **Goal**: collapse the 4 entities' soft-delete/undelete skeletons onto shared
  helpers; `authorize.rs` uses the module predicate.
- **Files**: `logic/soft_delete.rs` (new helper), `logic/{article,version,comment,user}.rs`,
  `logic/authorize.rs`.
- **Green**: each `delete_X(Soft)` / `undelete_soft_X` shrinks to
  authorize + helper + sync; `require_visible_if_soft_deleted` calls
  `soft_delete::is_hidden`.
- **Exit test**: `logic_article` + `logic_version` + `logic_comment`
  + `logic_user` + `logic_soft_delete_visibility` + `logic_delete_verify`
  + `logic_authorize` + `http_*` green + clippy + fmt.

### Slice 4 — Final gate + handoff

- Full suite (minus other agents' red probe files), clippy (0 warnings), fmt.
- Update `document/handoff/S7d4_soft-delete-state-machine.md` + readme index.
- Delete `document/exec/S7d4_soft-delete-state-machine.md` after green.

## Open Unknowns

- None requiring library source: the mechanism is fully source-visible in this
  repo (`hard_delete.rs`), and agdb API (DbValue, QueryBuilder::not().keys,
  transaction) is already exercised by the current code. A probe is still
  warranted to confirm the visibility predicate and cascade survive the move
  unchanged (see Verification Plan).

## Verification Plan

| Dimension | Method |
|---|---|
| Correctness | Existing suite (soft_delete_visibility, delete_verify, logic_authorize, search) + a new probe asserting `hide`/`restore`/`is_hidden` round-trip matches current semantics |
| Behavior change | Zero — compare suite results before/after each slice |
| Time complexity | Identical — same cascades, same query-time `.not().keys` filters preserved |
| Space complexity | Identical — same allocations, same key/value layout |
| Performance | Identical — no N+1 introduced; query-time batch filtering retained |

## Risks

- Splitting could subtly change an error path (e.g. "already soft-deleted" vs
  "not soft-deleted") — mitigated: slice 3 keeps exact messages; suite pins.
- `hide`/`restore` pre-checking the state separately could race — the current
  logic already checks-then-transitions in two DB reads; helper must preserve
  that exact sequence (no new atomicity assumption).
- `soft_deleted` key still referenced by seeded `data/agdb` — verified during
  slice 1 (schema.rs key move must match any seed/read references).
- Article.rs (551 lines) exceeds 512 — slice 2 replaces inline calls but does
  not grow it; if needed the conversion only shrinks lines.
- Other agent in-flight files — re-check git status before every build.

## Constraints

- No `unwrap`/`expect`/new panics; no comments; English only.
- Behaviour, messages, permissions, cascade, search behaviour unchanged.
- Never touch: harness.rs, probe_*.rs, review-logic-findings.md (other agent),
  frontend, Cedar schema/policy, transfer logic.
- One commit per slice; stage only own files; no amend/push/force.
- nightly + Cranelift + line-tables flags; `--test-threads=1`; no `--release`.

## Questions

1. **Module location (Q1)**: **RESOLVED — repository layer**. The state machine
   lives at `repository/soft_delete.rs`; a thin free-function helper in the
   logic layer handles the shared delete/undelete skeleton. (User 2026-08-19)
2. **Logic helper shape (Q2)**: **RESOLVED — free functions** taking
   `(db, kind, business_id, state_err)`. No red slices for pure moves.
   (User 2026-08-19)
3. **Scope (Q3)**: **RESOLVED — A+B+C + repository-only naming split**. The
   three delete semantics are cleanly separated at the repository layer only
   (`soft_delete.rs` + `hard_delete.rs` + `transfer.rs`); logic/interface/
   router/frontend keep `delete_X(mode)` dispatch and `DeleteMode` unchanged.
   Three-slice plan confirmed. (User 2026-08-19)
4. **Rename scope (Q4)**: **RESOLVED — repository only**. Rename/division
   stops at the repository layer; logic/interface/router/frontend untouched.
   (User 2026-08-19)