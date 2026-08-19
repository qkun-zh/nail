## Task IV: Unify all list-page response shapes into ListPage<T> (wire change B1–B6)

**Owner**: Qp3nT7
**Exec doc**: `document/exec/L4st_list-page-unify.md`
**Status**: COMPLETE — all 3 slices committed, handoff pending orchestrator final gate review

### A. Common + back producers + tests (slice 1)

1. **Status**: COMMITTED `fa24f69` — gate green: fmt, clippy 0, common 117/117, back 583/583 (logic 281, http 139, repository 107, infrastructure 45, configuration 11).
   - `code/common/src/response.rs`: added `ListPage<T> { items: Vec<T>, has_next: bool, total: u64 }` (same derives as before, no renames).
   - Deleted old page structs: `TagListPage`, `RoleListPage`, `UserListPage`, `VersionListPage`, `CommentListPage`, `SearchPage` (item types kept).
   - Back producers now return `ListPage<T>`: `logic/tag.rs`, `logic/role.rs`, `logic/user.rs`, `logic/version.rs`, `logic/comment.rs` (both read_comments + read_comment_children), `logic/search.rs` (search_articles).
   - `total` semantics: version/comment via new repo count helpers (`count_versions_of`, `count_comments_by_version`, `count_comment_children` — same filter as page queries, 0 when node missing/soft-deleted); search via pre-paginate `len()` (window-scoped; seekstorm `result_count_total` unreliable under `ResultType::Topk`).
   - `SearchPage.page` echo removed from back; frontend tracks the page locally (slice 2).
   - `interface/comment.rs` annotation → `ListPage<CommentView>` (only handler referencing a page type).
   - Tests: common round-trip test replaced with ListPage versions; logic/http tests updated to `items` + `total` assertions added (http article/comment/version/tag_apply).
   - Decisions (user approved): repo page-fn signatures stay 2-tuples; count helpers live beside the page readers; no wire renames; `.comments` on `SearchVersionItem` is nested, NOT a page — untouched.

### B. Frontend consumers + search page-echo removal (slice 2)

2. **Status**: COMMITTED `24d3262` — gate green: front 80/80 tests, check, fmt, clippy 0, `trunk build` OK.
   - Request wrappers: `request/{tag,role,user,version,comment,article}.rs` return `ListPage<T>`.
   - Pages: `page/tag/list.rs`, `page/article/tag_picker.rs`, `page/role/list.rs`, `page/user/list.rs`, `page/article/version/index.rs`, `page/article/version/comment.rs` + `comment/{index,detail,state}.rs` read `items`/`total`.
   - `page/article/search.rs`: `run_search` takes `requested_page`; on success sets `current_page`/`last_good_page` to it (server-echoed page == requested for all non-400 responses); error path still restores `last_good_page`.

### C. Final gate + handoff (slice 3)

3. **Status**: COMPLETE — final gate green: common 117, back 583 (per-module), front 80 + `trunk build`; fmt/clippy clean in all three crates. Handoff written; exec doc Change log updated.

### Notes for the user

- The single-process full back suite was OOM-killed by the system at load ~22 (8GB anon RSS on 9.7GB box). Per-module runs (the documented approach) pass consistently; not a code failure.
- `SearchVersionItem.comments` (nested, not a page) and URL-segment strings in front tests are intentionally unchanged.
- Pre-existing untracked rustc-ICE dumps in `code/back/` left untouched.
- No behavior change: wire payloads are byte-identical to before (items/total/has_next map 1:1 onto the old fields; search page echo replaced by client-side tracking).