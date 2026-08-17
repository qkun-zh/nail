# a3Kx — Search Author Link + User Public Page + Search Range Expansion

## 1. Requirement

**R1 (done)**: Author names in search results are clickable links to
`/public/user/{uid}`; a new `/public/user/{uid}` public page; all operation
buttons visible to everyone.

**R2 (new)**: Search supports role, article_id, version_id, comment_id,
author_id.

**Acceptance criteria**:
1. ✅ Article/comment author names in search results are links → `/public/user/{uid}`
2. ✅ `/public/user/{uid}` page shows id, name, email_hash, roles, articles
3. ✅ Existing operation buttons no longer hidden by login state
4. Search ranges include role, article_id, version_id, comment_id, author_id
5. Frontend search form shows the new search-range options

## 2. Scope

**In**: common types, back search index, back logic, front search rendering,
front user page, front button visibility, search range expansion
**Out**: do not remove /admin routes, do not change the permission model

## 3. Design Decisions

- store author_id in the search index (zero runtime cost, single startup rebuild)
- /public/user/{uid} page reuses admin detail-page logic
- operation buttons visible to everyone; frontend notifies when backend returns 403
- role stored as a comma-separated string (e.g. "admin,member"), searchable
- all ID fields change from store-only to indexable/searchable
- schema version "3"→"4", automatic rebuild at startup

## 4. Slice Breakdown

| Slice | Goal | Files |
|---|---|---|
| S1 | common: SearchArticleItem + SearchCommentItem add author_id | `common/src/response/search.rs` |
| S2 | back: search index add FIELD_AUTHOR_ID | `repository/search/schema.rs`, `document.rs`, `search.rs` |
| S3 | back: logic/search.rs passes author_id | `logic/search.rs` |
| S4 | front: search-result author names become links | `page/public/article/search/results.rs`, `comments.rs` |
| S5 | front: new /public/user/{uid} public page + route | `page/public/user.rs`, `router.rs` |
| S6 | front: remove login-based button hiding | `page/public/article/detail.rs` |
| S7a | common: SearchRange adds 5 variants | `common/src/search.rs` |
| S7b | back: make schema fields indexable + add FIELD_ROLE | `repository/search/schema.rs` |
| S7c | back: document.rs stores role | `repository/search/document.rs` |
| S7d | back: query.rs registers new fields | `repository/search/query.rs` |
| S7e | back: logic/search.rs parse_ranges registers | `logic/search.rs` |
| S7f | front: expand RANGE_KEYS/RANGE_LABELS | `page/public/article/search.rs` |
| S7g | test updates | `test/unit/common/search/tests.rs`, `test/unit/back/repository/search.rs` |

## 5. Open Unknowns

- automatic rebuild behavior after search-index schema-version bump — existing
  mechanism (`INDEX_SCHEMA_VERSION`), confirmed by source

## 6. Verification Plan

| Dimension | Method |
|---|---|
| Correctness | cargo test (513 back + 69 front) |
| Behavior change | trunk build + manual verification of search-result links |
| Time complexity | no new runtime cost (index storage) |
| Space complexity | ~36 bytes/document additional storage |
| Performance | no regression (index reads vs before) |

## 7. Risks

- search-index rebuild slightly slower on first startup — acceptable
- old index auto-deleted and rebuilt — existing mechanism

## 8. Constraints

- do not change the permission model
- do not remove /admin routes

## 9. Questions

None