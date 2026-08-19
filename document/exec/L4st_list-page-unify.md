# L4st — Task IV: unify list-page response shapes into `ListPage<T>`

## 1. Requirement

Replace the six ad-hoc page structs with one generic wire type:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPage<T> {
    pub items: Vec<T>,
    pub has_next: bool,
    pub total: u64,
}
```

Endpoints (approved wire change B1–B6):

| # | endpoint | old field(s) | new field(s) |
|---|---|---|---|
| B1 | tag list | `tag_list` | `items` |
| B2 | role list | `role_list` | `items` |
| B3 | user list | `user_list` | `items` |
| B4 | version list | `version_list`, `page` | `items`, `total` (page dropped) |
| B5 | article search | `article_list`, `page` | `items`, `total` (page dropped) |
| B6 | comment list | `comments` | `items`, `total` |

Old structs `TagListPage`, `RoleListPage`, `UserListPage`, `VersionListPage`,
`CommentListPage`, `SearchPage` are deleted (dead code rule D3). Item types
(`TagListItem`, `RoleListItem`, `UserListItem`, `VersionListItem`,
`SearchArticleItem`, `CommentView`) are unchanged.

Acceptance: all six endpoints serve `data.items`, `data.has_next`, `data.total`
(version/comment/search gain `total`; search/version drop the `page` echo); all
per-crate gates green; frontend search page still works with page tracked
locally.

## 2. Scope

In-scope: common definitions, back producers, repo count helpers, interface
annotation, frontend request/page consumers, all unit tests that read the old
field names, search page-echo removal.

Out-of-scope: pagination math (unchanged; `has_next` semantics preserved exactly
as today — repo `limit+1` fetch for version/comment, `paginate` div_ceil for
tag/role/user/search), item-type changes, `SearchVersionItem.comments` /
`SearchArticleItem.article_hits` etc. (nested fields, NOT pages), route
changes, other tasks (III/VIII/X).

## 3. Design decisions

1. `ListPage<T>` lives in `code/common/src/response.rs` (module root, next to
   `ResponseEnvelope`): both are shared generic wire types.
2. Derives identical to today: `Debug, Clone, PartialEq, Eq, Serialize,
   Deserialize`. No `rename_all`, no `skip_serializing_if` (fields are never
   absent).
3. `total` semantics — mirror tag/role/user exactly ("total item count before
   slicing"): they do `let total = collection.len() as u64;` then `paginate`.
   - version: new repo helper `count_versions_of` (same filter as `versions_of`
     page query: from article, distance 2, type version, not soft-deleted;
     returns 0 when article missing or soft-deleted, matching `versions_of`).
   - comment: new repo helpers `count_comments_by_version` /
     `count_comment_children` (same filter as `incoming_comment_ids_page`:
     to node, distance 2, type comment, not soft-deleted).
   - search: `let total = article_list.len() as u64;` before `paginate` in
     logic. This counts assembled articles in the current search window (docs
     are already offset/limited at the repo level); a true global hit count is
     not available accurately — seekstorm `result_count_total` is only accurate
     for `ResultType::TopkCount`/`Count`, and the repo uses `ResultType::Topk`
     (source: seekstorm-3.3.5 src/index.rs:197-198). Mandated mirror is
     pre-slice count; documented here as accepted semantics.
4. Repo API for `versions_of` / `read_comments_page_by_version` /
   `read_comment_children_page` stays a 2-tuple; count helpers are separate
   functions. Changing the tuple shape would churn 19+ test call sites for no
   wire benefit.
5. Search `page` echo removal: the frontend already has `current_page` +
   `last_good_page` signals. `run_search` gains a `requested_page` param; on
   success set `current_page`/`last_good_page` to it. For every non-400
   request the server-echoed page equals the requested page (server rejects
   `page > max_pages` with 400 and otherwise returns the requested page), so
   behavior is preserved for all valid requests.
6. Local `Vec` names (`tag_list`, `role_list`, ...) in logic producers renamed
   to `items` for consistency; field access is what matters.
7. Slice atomicity: front depends on `nail_common` via path; removing the old
   structs breaks front compilation until slice 2. Accepted (orchestrator's
   slice order); tree is fully green only after slice 2.

## 4. Slice breakdown

- Slice 1 — backend: common `ListPage<T>` + delete old structs + repo count
  helpers + logic producers + interface annotation + common/back tests.
  Exit: common 116+ tests green; back 583+ tests green (per-module).
- Slice 2 — frontend: request/page consumers + search page-echo removal.
  Exit: `cargo +nightly check` in code/front.
- Slice 3 — final gate: `fmt --check` + `clippy -D warnings` + full per-crate
  tests + front check; delete exec doc; handoff.

## 5. Open unknowns (evidence)

| Unknown | Evidence |
|---|---|
| Where page structs are referenced | grep inventory (below) — source |
| `total` semantics today | logic/tag.rs:41, logic/role.rs:59, logic/user.rs:120 — source |
| seekstorm total availability | seekstorm-3.3.5 src/index.rs:197-198 — source |
| version/comment repo page filter | repository/version.rs:173-222, repository/comment.rs:256-292 — source |
| search assembly + double pagination | logic/search.rs:75-82 — source |
| frontend page tracking | page/article/search.rs:95,159,178-179,196-228 — source |

No probe needed: all behavior is visible in repo source (no crate-API
uncertainty beyond seekstorm, which is documented source evidence).

## 6. Verification plan

- Correctness: back per-module tests (http asserts `data.items`; logic tests
  assert `.items`); common round-trip + wire-shape test.
- Behavior change: input/output delta vs baseline = exactly B1–B6 (only field
  renames + added `total` + dropped `page`). Pagination math untouched
  (diff review).
- Time/space complexity: version/comment add one count query per list request
  (same filter as page query; O(full-set) on the graph search, previously
  O(page) — accepted, matches tag/role/user in-memory full-scan semantics).
  Search adds zero queries (in-memory len).
- Performance: no latency regression measured (count query is a single indexed
  graph search; small result sets per article/comment).

## 7. Risks

- Missing a consumer → compile error; mitigated by exhaustive grep inventory
  below and per-crate gates.
- `total` on version/comment = separate query could disagree with page under
  concurrent mutation (pre-existing race for tag/role/user too; accepted).
- Front not compiling between slice 1 and 2 (see decision 7).
- Search `total` is window-scoped (decision 3) — documented, frontend does not
  display it.
- Rollback: revert the slice commits (linear, one commit per slice).

## 8. Constraints

- No `unwrap`/`expect`/new panics in production. No comments restating code.
  English only. No hand-edited Cargo.lock. Read/Edit/Write only (no
  sed/awk/cat>). Check load before every build; per-module test runs, serial.
- Only the declared file list; anything else flagged to orchestrator.

## 9. Questions

None — the orchestrator's B1–B6 spec is unambiguous; decisions 3/5 resolve the
only judgment calls and are evidence-backed.

## Consumer inventory (every old-name read/type use)

### A. Common definitions
- `code/common/src/response.rs` — add `ListPage<T>`.
- `code/common/src/response/tag.rs:17-22` — delete `TagListPage`.
- `code/common/src/response/role.rs:19-24` — delete `RoleListPage`.
- `code/common/src/response/user.rs:36-41` — delete `UserListPage`.
- `code/common/src/response/version.rs:17-22` — delete `VersionListPage`.
- `code/common/src/response/comment.rs:16-20` — delete `CommentListPage`.
- `code/common/src/response/search.rs:41-46` — delete `SearchPage`.

### B. Back producers
- `code/back/src/logic/tag.rs:14,38,53-57`
- `code/back/src/logic/role.rs:12,56,74-78`
- `code/back/src/logic/user.rs:5,117,132-136`
- `code/back/src/logic/version.rs:4-6,176,193-197`
- `code/back/src/logic/comment.rs:4,77,96,124,144`
- `code/back/src/logic/search.rs:5,20,78-82`
- `code/back/src/repository/version.rs` — add `count_versions_of`.
- `code/back/src/repository/comment.rs` — add `count_comments_by_version`,
  `count_comment_children`.

### C. Back interface
- `code/back/src/interface/comment.rs:5,85` — `CommentListPage` →
  `ListPage<CommentView>` (only handler referencing a page type; tag/role/user/
  version/article handlers pass `data` through, verified).

### D. Front request
- `code/front/src/request/tag.rs:3,9`
- `code/front/src/request/role.rs:4,10`
- `code/front/src/request/user.rs:8,24`
- `code/front/src/request/version.rs:2,12`
- `code/front/src/request/comment.rs:2,12,31`
- `code/front/src/request/article.rs:3,9`

### E. Front pages
- `code/front/src/page/tag/list.rs:4,8,30,39`
- `code/front/src/page/article/tag_picker.rs:13`
- `code/front/src/page/role/list.rs:3,9,31`
- `code/front/src/page/user/list.rs:4,8,30`
- `code/front/src/page/article/version/index.rs:5,15,57-60`
- `code/front/src/page/article/search.rs:176,178-179` (+ run_search signature)
- `code/front/src/page/article/version/comment.rs:20,49,51`
- `code/front/src/page/article/version/comment/index.rs:3,11,21`
- `code/front/src/page/article/version/comment/detail.rs:3,12,45`
- `code/front/src/page/article/version/comment/state.rs:7,18,20`

### F. Tests
- `test/unit/common/response/tests.rs:101-112` — replace SearchPage round-trip
  with `ListPage<T>` round-trip + wire-shape asserts.
- `test/unit/back/http/tag_apply.rs:39`
- `test/unit/back/http/role.rs:49,116` (total assert 123 unchanged)
- `test/unit/back/http/version.rs:160` (+ optional total assert)
- `test/unit/back/http/article.rs:751,753`
- `test/unit/back/http/comment.rs:142,144,263`
- `test/unit/back/logic/role.rs:80,118,152`
- `test/unit/back/logic/comment.rs:134,144,346`
- `test/unit/back/logic/delete_verify.rs` — `.comments`:162,201,205,384,813;
  `.version_list`:303; `.article_list`:108,353,355,638,772
- `test/unit/back/logic/pagination_verify.rs` — `.version_list`:132,141,180,
  182,184,198,222,252,297,298,332,333,597; `.comments`:361,363,383,420,421,461,
  463,486; `.article_list`:576,578,620; type ann:251
- `test/unit/back/logic/search.rs` — `.article_list`:171,228,232,279,283,285,
  287,374,386,390,399,408,418,435,450,468,487,491,548,561,589,592,630,634,678,
  682,728,730,753,758,773,778,793,798,830,833
- `test/unit/back/logic/search_verify.rs` — `.article_list`:96,112,147,190,251,
  304,312,360,374,409,429,435,458,496,503,530,532,537,561,587,617,626,665,705,
  729,739,749,785,823,837,860,861,870,900,901,930,974,1010,1012,1054,1088,1092,
  1138,1182,1186; type ann:428

### NOT changed (verified distinct from page fields)
- `SearchVersionItem.comments` (nested): search_verify.rs:364,1101,1103;
  `code/front/src/page/article/search/versions.rs:22`.
- `test/unit/front/request/url/tests.rs:35`, `test/unit/front/page/comment/url/
  tests.rs` — URL-segment strings, not response fields.
- `versions_of`/comment-page repo callers in repository tests (repo API
  unchanged): repository/delete.rs, repository/version.rs, repository/comment.rs,
  http/article.rs:580, logic/version.rs:258, logic/article.rs:292,471,
  logic/search_verify.rs:229, logic/delete_verify.rs:87,711,806,878.

## Change log

- 2026-08-19: created.