# Handoff

## Task V: Repository graph abstraction (Xz9v)

**Owner**: Wb5Kq2
**Exec doc**: `document/exec/Xz9v_repo-graph-abs.md`
**Status**: Complete — all 7 slices done, red-first Stage C fix proven, final gate green (543/543)

### Stages

- A. ✅ Centralized typed-edge query blocks (~30 sites) into
  `graph::outgoing_edges`/`incoming_edges` (Slices 1-5)
- B. ✅ One definition per concept: `resolve_node_id`/`find_by_index`/`read_rows`/
  `read_node`/`has_soft_deleted_flag` over `GraphQuery`; all `_in_txn` pairs and
  `read_node_sync` gone (Slices 1-5)
- C. ✅ Semver-correct `graph::highest_version_number` shared by version.rs /
  article.rs / delete.rs; delete.rs string-max bug fixed red-first (Slice 7)
- D. ✅ All edge insertion via `graph::insert_edge` (`&mut impl GraphWrite`)
  (Slices 2, 4, 5)
- E. ✅ `read_tag_articles` removed from production; no `#[allow(dead_code)]`
  remains in repository/ (Slice 6)

### Evidence (red-first, Stage C)

- New repro `delete_refresh_keeps_the_semver_latest_version`
  (`test/unit/back/repository/delete.rs`): article versions "1.0.0" (id
  ffffffff-…), "9.9.9" (id 11111111-…), "10.0.0" (id 22222222-…); delete
  "10.0.0".
- **RED (observed pre-fix)**: `assertion failed: left: "1.0.0" right: "9.9.9"`
  — `.map(|row| row.id).max()` compared version *business-id* strings.
- **GREEN (post-fix)**: `highest_version_number` = semver `max_by` with string
  fallback (matches article.rs's prior fallback); wired into delete.rs
  `refresh_latest_version_in_txn`, version.rs `create_version` (strict
  `InvalidNumber` pre-validation preserved), article.rs `live_latest_version`.

### Decisions / deviations

- `read_tag_articles` is used by 4 live tests (`test/unit/back/logic/
  tag_apply.rs` L72/91/110/126) — test-support, not dead. Removed from
  production; kept as `#[cfg(test)]` copy; tests untouched.
- `read_tag_detail` (`logic/tag.rs`, out of scope) removed — sole production
  consumer of `read_tag_articles`; another task re-adds it when wiring the
  feature.
- `resolve_node_id_sync`/`find_by_index_sync`/`read_rows_sync` stay in
  graph.rs — pinned by out-of-scope `user.rs`; all other old helpers removed.
- Full `cargo test` OOMs on this machine (9GB RAM) — per-module runs used
  (`repository_`/`logic_`/`http_`/`infrastructure_`/`configuration_`).

### Code changes (one commit per slice)

- `c840e67` exec doc · `29f56aa` slice 1 graph+delete · `d203237` slice 2
  version+tag · `15a4681` slice 3 article · `6b49a93` slice 4 comment+role ·
  `8f5a9d4` slice 5 transfer+authorization+search · `9b6586e` slice 6 Stage E ·
  `b6fbd94` slice 7 Stage C fix + repro

### Final gate

- ✅ `cargo fmt --check` — clean
- ✅ `cargo +nightly clippy -- -D warnings` — zero warnings
- ✅ back tests per module — 107 + 260 + 122 + 43 + 11 = **543/543 pass**
  (baseline 542 + Stage C repro)

### Open questions

- None for this task. `user.rs` migration to the unified helpers is a natural
  follow-up (was out of scope).