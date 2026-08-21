# Handoff

## Task VI: Database crate extraction (D7kq)

**Owner**: Q8mVz4
**Exec doc**: `document/exec/D7kq_database-crate.md`
**Status**: Slices 1-2 committed and CI-green; slice 3 (authorizer crate) next

### Stages

- A. ✅ Research + adopted API contract (`document/database-crate-research.md`,
  commit 0fe1a06)
- B. ✅ Slice 1 — standalone `code/database` crate: scoped transactional API
  (`Database::read/write` → `ReadScope`/`WriteScope`), `Row`/`ValueLookup`,
  `NodeKind`/`EdgeKind`, `Condition`/`Order` pushdown, 22 unit tests
  (commit 07089a0)
- C. ✅ Slice 2 — atomic back migration: all repositories take `&Database`
  and are synchronous; `DbHandle`/agdb removed from `back`;
  `repository/graph.rs` deleted; address parsing relocated to
  `infrastructure::server::open_database` (commit 16e2453, CI success)
- D. Slice 3 — extract `authorizer` crate (pending)
- E. Slice 4 — extract `searcher` crate (pending)

### Key decisions (slice 2)

- `GraphRead` trait (`back/src/repository/access.rs`) with `scope_*`-prefixed
  methods lets internal helpers be generic over read/write scopes.
- Write closures whose domain errors differ from `database::Error` return
  two-layer `Result<Result<T, DomainError>, Error>`; outer fn flattens via
  `.map_err(DomainError::from).and_then(std::convert::identity)`.
- Business id persisted under the `id` key on insert/replace
  (`database::ID_KEY`) so row projections recover it; alias stays the
  uniqueness mechanism.
- `find_by_key` is kind-agnostic (global indexes); callers verify rows.
- Logic layer de-asynced where awaits only wrapped repositories (clippy
  `unused_async`): authorizer.authorize, logic authorize family, role/tag/
  user/version/comment read paths, download token fns.
- Migration bug fixes vs first cut: comment pager edge kind parameterized
  (replies were invisible); `users_holding_role` reads `IdRow` not `RoleRow`;
  `read_comment_item` keeps no soft-delete filter at repository layer.

### Verification

- database 22 + back 570 + common 81 tests pass locally (`-j 1`).
- clippy pedantic `-D warnings` clean workspace-wide; fmt clean.
- CI run #32448026075 (16e2453): success.

### Remaining risks / notes

- `seed.rs` rewrite verified faithful against git HEAD original (index
  creation moved into `Database::open_*` via `INDEX_KEYS` by design).
- Full local `--workspace` test runs are slow/OOM-prone (9GB RAM) — rely on
  CI for cross-crate gates per workflow §8.
