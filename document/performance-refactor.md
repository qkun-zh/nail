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

**Approved? NO — OPEN.** This changes the index shape and the search highlight
granularity (per-version/per-comment hits), i.e. observable behavior the user has not
agreed to change. Requires an explicit user decision (see "Open decisions").

## P2. `enrich_articles` full-graph scan — O(E) time and space for a single read

Location: `code/back/src/repository/article.rs:313-415`

Problem: `read_article` calls `enrich_articles`, which scans the **entire**
`EDGE_USER_AUTHOR_ARTICLE` and `EDGE_ARTICLE_APPLY_TAG` edge tables
(`article.rs:325,346`, `.search().elements()` with no `from`/`to` filter), then
filters by `node_set`. Cost is O(total edges) instead of O(1).

Library evidence (verified):
- agdb `where_` has **no** `from`/`to` endpoint filter (only `ids`/`key`/`distance`/
  `beyond`; `where_.rs`). Its `ids` condition matches the **element's own db id**
  (`db.rs:1213-1227`) — for an edge search that is the edge id, not its endpoints. It
  also is not composable with `.key()` (returns `WhereLogicOperator`, no `.key`). So
  `where_.ids(article_ids)` is **not** a valid localization (refutes the old plan).
- Targeted `to`/`from` exist and are used in-repo (`version.rs:236`, `comment.rs:198`,
  `document.rs:141`).

Probe evidence (`test/unit/back/repository/probe.rs`, probe passed): with two articles,
the current graph scan sees 2 owner + 2 tag edges; targeted `.to(a1)` returns exactly the
1 owner edge and `.from(a1)` the 1 tag edge, and the returned **edge ids are identical**
(assert_eq) to the current scan+filter for `a1`. Behavior preserved.

Planned solution (revised): since `enrich_articles` is only ever called from `read_article`
with a single id (`article.rs:190`), use targeted `.search().to(article)` for the owner
edge and `.search().from(article)` for the tag edges, plus batch `read_rows`. O(E) → O(1).

**Approved? YES — IMPLEMENTED (commit 20fdeb4).** No observable behavior change
(probe-verified identical edge set); no risk; reaches the theoretical optimum for the
single-article read. Verification: probe plus existing article-read tests, 305 tests pass.

## P3. `enrich_comment_headers` per-comment round trips — O(C) time

Location: `code/back/src/repository/search.rs:424-441`

Problem: for each of C comment hits, 3 separate graph queries are issued (article
title, article author, version number), and the author lookup traverses the owner edge
each time. O(C) round trips.

Library evidence: agdb `read_rows_sync(&ids)` batch reads and targeted edge queries
resolve many articles/versions in a handful of queries (see P2 evidence).

Planned solution: collect all article ids and version ids, resolve and `read_rows_sync`
them in batch; fetch authors with one targeted edge query. O(C) → constant number of
queries. Output is identical.

**Approved? YES.** No observable behavior change (same enriched comment fields); no
risk; reaches the theoretical optimum (bounded batch queries instead of O(C) round
trips). Verification: existing search tests plus a new TDD test asserting unchanged
comment enrichment.

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
`[offset, offset+limit)`.

Library evidence (verified): agdb `LimitOffsetHandler` (`db_search_handlers.rs:134-151`)
walks storage-slot order and `Finish` once it has counted `limit + offset` matches. A
listing with **no `order_by` short-circuits at O(offset+limit)** instead of a full
scan + sort.

Planned solution: drop the ordering for the short-circuit (O(n log n) → O(offset+limit)).

**Approved? NO — OPEN.** Removing the order changes observable behavior: the code
currently sorts versions newest-first by id descending (`version.rs:214`). Whether
newest-first is required is a product call the user must make.

## P6. `pick_recycler_target` — O(R²) dedup

Location: `code/back/src/repository/transfer.rs:222-250`

Problem: for each recycler, `exclude.contains(&user_id)` scans the exclude list, so R
recyclers cost O(R²).

Planned solution: replace `exclude.contains` with a `HashSet<String>` of excluded ids
(O(1) membership). Selection result is identical (same excluded set, same best-pick).

**Approved? YES.** No observable behavior change (the recycler chosen is unchanged);
no risk; reaches the theoretical optimum (O(R) instead of O(R²)). Verification: existing
transfer/recycler tests plus a new TDD test asserting the same selection with a
duplicate-laden exclude list.

## Open decisions for the user (behavior-changing, not yet approved)

- **Keep `total` vs cursor.** Keeping `total` costs an O(A) full scan on agdb *list*
  endpoints; the search-page `total` is free (SeekStorm, no agdb scan). Dropping it for
  cursor is viable only if jump-to-page is not a product need.
- **Master-doc-per-article (P1) vs version/comment-indexed.** The current design is
  version+comment granular, which keeps highlight cost O(single field) but makes article
  paging O(offset). Master-doc reverses it: cheap article paging, loses per-version/
  comment highlight, risks article-level bloat.
- **No ORDER BY for search.** Only helps the empty-query/browse path
  (`search_iterator_index`, `search.rs:1413-1432`); keyword search still ranks by BM25.
- **No ORDER BY for listing (P5).** Verified `LimitOffsetHandler` short-circuit, but it
  drops newest-first ordering.

## Decision status

| Item | Status |
| --- | --- |
| P1 deep pagination (master doc) | **Open** — behavior change (index shape + highlight) |
| P2 enrich_articles localize (targeted `to`/`from`) | **Done** (commit 20fdeb4) |
| P3 enrich_comment_headers batching | **Approved** — implement |
| P4 sync_all | Accepted as inherent (no change) |
| P5 pagination sort + slice | **Open** — drops newest-first ordering |
| P6 recycler O(R²) → HashSet | **Approved** — implement |
| Search: no ORDER BY / cursor / no total | **Open** — user decides |

_Last updated: P2 implemented (commit 20fdeb4). P3/P6 approved pending evidence+implementation;
P1/P5/total/cursor open. Baseline: build green, 305 tests pass._

## Probe evidence log

| # | Probe | Result |
| --- | --- | --- |
| P2 | `test/unit/back/repository/probe.rs::probe_targeted_queries_localize_by_endpoint` | Passed. Graph 2 owner+2 tag edges; targeted `.to(a1)/.from(a1)` return identical edge ids to scan+filter. Refutes `where_.ids`. |