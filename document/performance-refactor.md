# Performance Refactor

Tracking the identified algorithmic bottlenecks in the backend business logic, the
planned solutions (with library-source evidence), and approval status. Updated on
every plan change.

Approval gate (user-set): a solution is **approved** only if it (1) carries no risk,
(2) reaches the theoretical optimum, and (3) does **not** change behavior the user has
not agreed to change. Anything that alters observable behavior stays open for the user
to decide.

## Context

Covers the backend search and repository paths in `code/back`. Evidence is from the
pinned crate sources in the local cargo registry: `seekstorm-3.3.5`, `agdb-0.13.2`.

Baseline (before implementation): build green, `cargo test` 304 passed, clippy
zero-warning pending re-run.

## P1. Deep search pagination — O(offset) time and space

Location: `code/back/src/repository/search.rs:277`, `code/back/src/logic/search.rs:76-99`

Problem: a page is served by fetching `top_k = offset + limit * MAX_DOCS_PER_ARTICLE`
(×32) documents from the index, then `get_document` + `enrich` + `assemble_tree`
post-process **all** of them before slicing. Both time and space scale with `offset`,
not `limit`.

Library evidence:
- `IndexArc::search` supports a native `offset` (`search.rs:1134-1150`); it keeps a
  heap of `min(offset + length, indexed_doc_count)` (`search.rs:2527-2528`) and
  internally `split_off(offset)` + `truncate(length)` (`search.rs:2109-2119`).
- Root cause of the ×32: the index is **document-granular** (1 article → up to 32
  version/comment docs) while the business pages by **article**.

Planned solution:
1. Make the index article-granular — one master/representative document per article.
2. Call `index.search(offset, limit)` natively; post-processing touches only `limit`
   docs and `assemble_tree` becomes trivial.

**DECIDED — NOT DOING IT.** User rejected the master-doc fix: it changes the search
highlight granularity (per-version/per-comment hits → article-level), and the user does
not want that behavior changed. The O(offset) pagination cost is therefore accepted as-is.
No code change for P1.

## P3. `enrich_comment_headers` per-comment round trips — O(C) time

Location: `code/back/src/repository/search.rs:424-441`

Problem: for each of C comment hits, 3 separate graph queries are issued (article
title, article author, version number), and the author lookup traverses the owner edge
each time. O(C) round trips.

Library evidence: agdb `read_rows_sync(&ids)` (`graph.rs:128`) batch-reads a slice of node
ids in **one** query; `resolve_node_id_sync` (`graph.rs:36`) resolves one business id via a
direct alias select (O(1) each). Targeted `.to(article)` owner-edge queries exist
(`article.rs`, `version.rs:236`).

Probe evidence (`test/unit/back/repository/probe.rs::probe_batch_comment_enrichment_matches_per_comment`,
passed): with 2 articles (2 versions, 1 shared author) and 4 fabricated `(article_id,
version_id)` comment pairs — including a duplicate and a cross article/version pair — the
batch path (resolve each distinct id once, `read_rows_sync` the article/version/user rows
in bulk, one targeted owner-edge query per distinct article) produces the **identical**
`(article_title, article_author_name, version_number)` tuple as the current per-comment
helpers, per pair (assert_eq). Behavior preserved.

Planned solution: collect the distinct article ids and version ids from all comments;
resolve each once and batch `read_rows_sync` the `ArticleRow`/`VersionRow`/`UserRow`;
fetch authors with one targeted owner-edge query per distinct article; look up by node
per comment. O(C) round trips → O(#distinct articles + #distinct versions) resolves plus
a constant number of batch reads. Output is identical.

**Approved? YES — IMPLEMENTED (commit cf701c4).** No observable behavior change
(probe-verified identical enriched comment fields); no risk; reaches the theoretical
optimum (bounded batch queries instead of O(C) round trips). Verification: probe plus
existing search tests, 306 tests pass.

## P4. `sync_all` full rebuild — O(total documents) time and space

Location: `code/back/src/repository/search.rs:185-230`

Problem: `sync_all` materialises the documents for **every** article into one `Vec`
before indexing (`search.rs:218-224`). Space is O(total corpus docs).

Planned solution: none — this is inherent to "rebuild everything". The library provides
no per-article streaming batch here.

**Approved? N/A — accepted as inherent cost.** No change planned.

## P5. Pagination sorts the full set before slicing — O(n log n)

Location: `code/back/src/repository/version.rs:180-221`, `code/back/src/repository/comment.rs:218-262`

Problem: `versions_of`, `read_comments_page_by_version` and
`read_comment_children_page` load **all** sibling ids, sort them O(n log n), then slice
`[offset, offset+limit)`. They also compute `total` (a full count).

Library evidence (verified): agdb `.offset(offset).limit(limit)` on a search query
short-circuits and returns once `limit + offset` matches are hit
(`query_builder/search.rs:542-543`); requesting **ordering disables that short-circuit**
(`:544-545`), so dropping the order is what unlocks it. Default order is storage order.
`SelectOffset::limit` / `.offset().limit()` chain documented at `:552`.

Probe evidence (`test/unit/back/repository/probe.rs::probe_offset_limit_pagination_tiles_default_order`,
passed): 4 versions on one article; paging with `.from(article).offset(offset).limit(limit)`
(no sort) tiles the full default-order set with no gaps/overlaps/duplicates, and `has_next`
is determinable by fetching `limit+1`. Verification: full test suite green.

Planned solution (user-approved): drop the newest-first sort and the `total` count
everywhere (frontend + backend). `versions_of` and both comment paginators issue a single
`.offset().limit()` query over the sibling edges (default order), derive `has_next` from a
`limit+1` peek, and return no `total`. O(n log n) + full count → O(offset+limit).

**Approved? YES — IMPLEMENTED (commit c2f62ec).** User decision: drop newest-first order
and `total`; use default (storage) order. `versions_of` and both comment paginators now
issue a single `.offset().limit()` query over sibling edges (default order), derive
`has_next` from a `limit+1` peek, and return no `total`. DTOs drop `total` (and
`VersionListItem.created_at`, unused). Frontend uses a new `PrevNext` control instead of
numbered pagination. 308 tests pass; frontend trunk build clean.

## P6. `pick_recycler_target` — O(R²) dedup

Location: `code/back/src/repository/transfer.rs:222-250`

Problem: for each recycler, `exclude.contains(&user_id)` scans the exclude list, so R
recyclers cost O(R²).

Library evidence: `Vec::contains` is a linear scan (std `slice` `contains`,
`core/src/slice/mod.rs:2589`); `HashSet::contains` is amortized O(1) (std hashbrown).
Converting the exclude slice to a `HashSet` once makes each membership test O(1).

Probe evidence (`test/unit/back/repository/probe.rs::probe_recycler_selection_hashset_matches_vec_exclude`,
passed): with 3 candidates of distinct workload (r1=2 articles, r2=1, r3=0) and an
exclude list `[r1, r1, r3]` (duplicate + mixed), membership is equal per candidate and the
chosen recycler is **identical** (r2) whether membership is tested via `Vec::contains` or a
built `HashSet`. Behavior preserved.

Planned solution: replace `exclude.contains` with a `HashSet<String>` built once from the
exclude slice (O(1) membership). Selection result is identical (same excluded set, same
best-pick).

**Approved? YES — verified, then implemented per user directive ("先验证p6，通过后可做").**
No observable behavior change (probe-verified identical recycler choice); no risk; reaches
the theoretical optimum (O(R) instead of O(R²)).

## Open decisions for the user (behavior-changing, not yet approved)

- **Keep `total` vs cursor on agdb *list* endpoints.** Keeping `total` costs an O(A) full
  scan; dropping it for cursor is viable only if jump-to-page is not a product need. The
  search-page `total` is already dropped — it was not a true count (see footer).
- **Master-doc-per-article (P1) vs version/comment-indexed.** The current design is
  version+comment granular, which keeps highlight cost O(single field) but makes article
  paging O(offset). Master-doc reverses it: cheap article paging, loses per-version/
  comment highlight, risks article-level bloat.
- **No ORDER BY for listing (P5).** Verified `LimitOffsetHandler` short-circuit, but it
  drops newest-first ordering.

## Decision status

| Item | Status |
| --- | --- |
| P1 deep pagination (master doc) | **Closed** — rejected by user (would change highlight) |
| P3 enrich_comment_headers batching | **Done** (commit cf701c4) |
| P4 sync_all | Accepted as inherent (no change) |
| P5 pagination sort + slice | **Done** (commit c2f62ec) — drop sort + total, default order |
| P6 recycler O(R²) → HashSet | **Done** (verified + user-approved) |
| Search: no ORDER BY | **Done** (commit af09b00) — drop time/title/author sort, default relevance order; removed `SearchSort*` common types + sort UI |
| Search: no total | **Done** — drop `total`/`total_pages`/`has_prev`/`truncated` from `SearchPage`; `has_next` derived from the assembled top_k window; frontend uses prev/next |
| List endpoints: keep total | **Open** — O(A) full scan; search-page total no longer the free case |

_Last updated: search total removed — `SearchPage` is now `{article_list, page, has_next}`.
Search `total` was NOT a true count: `total = article_list.len()` over docs assembled from
the top_k window (`top_k = offset + limit*32`, `repository/search.rs:254`, `logic/search.rs`),
which SeekStorm caps via `results.truncate(length)` (`seekstorm search.rs:2118`);
`result_count_total` is only accurate for `TopkCount` (`:197`). Probe: 34 matching articles,
limit=1 -> reported total 32, `has_next` true but window-exhausted -> total drifted per page
and `total_pages`/`truncated` were unreliable. Removing it drops the truncated-warning and
numbered pagination (now prev/next). Follow-up: `server.max_search_pages` config is now a
dead knob (was only consumed by the removed truncated warning). P1 rejected (highlight
behavior), P4 accepted (inherent), P6 non-problem (O(R), exclude length 1). Baseline:
build green, 311 back tests + 109 common + 69 front pass._

## Probe evidence log

| # | Probe | Result |
| --- | --- | --- |
| P3 | `probe.rs::probe_batch_comment_enrichment_matches_per_comment` | Passed. 4 comment pairs (2 distinct articles, 2 versions, 1 author; incl. duplicate + cross pair): batch path returns identical (title, author, version) to per-comment. |
| P5 | `probe.rs::probe_offset_limit_pagination_tiles_default_order` | Passed. 4 versions; `.offset().limit()` (no sort) tiles default-order full set, no gaps/overlaps/dups; `has_next` via limit+1 peek. |
| Search total (pre-removal) | `probe_search_total_truncates_when_matches_exceed_topk` | Passed. 34 matching articles, limit=1 -> top_k=32 -> reported total 32 (true 34). Proves `total` is a truncated top_k-window count, not a real count. |
| Search no total | `probe.rs::probe_search_has_next_pages_cover_all_matches_without_total` | Passed. limit=1 over 34 articles; prev/next-style paging via `has_next` alone collects all 34 with no gaps/dups. |