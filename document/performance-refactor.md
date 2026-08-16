# Performance Refactor

Tracking the identified algorithmic bottlenecks in the backend business logic, the
solutions planned (with library-source evidence), and the current status. Updated
on every plan change.

## Context

These notes cover the backend search and repository paths in `code/back`. Only
suboptimal (non-theoretically-optimal) operations are tracked. Evidence is from
the pinned crate sources in the local cargo registry:

- `seekstorm-3.3.5`
- `agdb-0.13.2`

## Current problems and planned solutions

### P1. Deep search pagination — O(offset) time and space

Location: `code/back/src/repository/search.rs:277`, `code/back/src/logic/search.rs:76-99`

Problem: a page is served by fetching `top_k = offset + limit * MAX_DOCS_PER_ARTICLE`
(×32) documents from the index, then `get_document` + `enrich` + `assemble_tree`
post-process **all** of them before slicing. Both time and space scale with
`offset`, not `limit`.

Library evidence:
- `IndexArc::search` supports a native `offset` (`search.rs:1134-1150`); it keeps a
  heap of `min(offset + length, indexed_doc_count)` (`search.rs:2527-2528`) and
  internally `split_off(offset)` + `truncate(length)` (`search.rs:2109-2119`).
- Root cause of the ×32: the index is **document-granular** (1 article → up to 32
  version/comment docs) while the business pages by **article**.

Planned solution (proposed, not yet implemented):
1. Make the index article-granular — one master/representative document per article
   — so article count == document count.
2. Then call `index.search(offset, limit)` natively; post-processing touches only
   `limit` documents and `assemble_tree` becomes trivial (or disappears).
This removes both the O(offset) time and O(offset) space. Trade-off: per-version and
per-comment hit highlighting must be recovered by re-querying details from the master
hit.

Status: proposed. Scope is significant (index shape + search result assembly).

### P2. `enrich_articles` full-graph scan — O(E) time and space for a single read

Location: `code/back/src/repository/article.rs:313-415`

Problem: `read_article` (a single article) calls `enrich_articles`, which scans the
**entire** `EDGE_USER_AUTHOR_ARTICLE` and `EDGE_ARTICLE_APPLY_TAG` edge tables
(`article.rs:325,346`, `.search().elements()` with no `from`/`to` filter), then
filters by `node_set`. Cost is O(total edges) instead of O(1).

Library evidence:
- agdb supports targeted queries: `.search().to(id)` (`version.rs:236`, `comment.rs:198`)
  and `.search().from(id)` (`version.rs:190`, `document.rs:141`).
- Batch reads: `read_rows_sync::<T>(guard, &ids)` accepts many ids in one query
  (`article.rs:204,340`, `version.rs:204`).

Planned solution (chosen — salvaged from `decisions.md`): query the author/tag edges
with `where_.ids(article_ids)` and batch `read_rows`, giving O(page·degree) instead of
O(A+edges). Localize the enrich so single reads no longer scan all edges.

Status: proposed.

### P3. `enrich_comment_headers` per-comment round trips — O(C) time

Location: `code/back/src/repository/search.rs:424-441`

Problem: for each of C comment hits, 3 separate graph queries are issued (article
title, article author, version number), and the author lookup also traverses the
owner edge each time. O(C) round trips.

Library evidence: agdb `read_rows_sync(&ids)` batch reads and targeted edge queries
can resolve all articles/versions in a handful of queries (see P2 evidence).

Planned solution: collect all article ids and version ids, resolve and `read_rows_sync`
them in batch; fetch authors with one targeted edge query. O(C) → constant number of
queries.

Status: proposed.

### P4. `sync_all` full rebuild — O(total documents) time and space

Location: `code/back/src/repository/search.rs:185-230`

Problem: `sync_all` materialises the documents for **every** article into one `Vec`
before indexing (`search.rs:218-224`). Space is O(total corpus docs).

Planned solution: none — this is inherent to "rebuild everything". Out of scope unless
a per-article streaming index batch is added (library does not provide one here).

Status: accepted as inherent cost (no change planned).

### P5. Pagination sorts the full set before slicing — O(n log n)

Location: `code/back/src/repository/version.rs:180-221`, `code/back/src/repository/comment.rs:218-262`

Problem: `versions_of`, `read_comments_page_by_version` and
`read_comment_children_page` load **all** sibling ids, sort them O(n log n), then
slice `[offset, offset+limit)`.

Library evidence (verified): agdb `LimitOffsetHandler` (`db_search_handlers.rs:134-151`)
walks storage-slot order and `Finish` once it has counted `limit + offset` matches. So a
listing query with **no `order_by` short-circuits at O(offset+limit)** instead of a full
scan + sort.

Open tradeoff (not decided — salvaged, but is a cost): the current code sorts versions by
id descending (`version.rs:214`), i.e. newest-first. Removing the order for the
short-circuit trades that UX for O(offset+limit) (deep pages still O(offset)). Whether
newest-first is required is a product call, not a settled fact.

Status: identified; solution = drop ordering for the short-circuit, pending the UX call.

### P6. `pick_recycler_target` — O(R²) dedup

Location: `code/back/src/repository/transfer.rs:222-250`

Problem: for each recycler, `exclude.contains(&user_id)` scans the exclude list, so
R recyclers cost O(R²).

Planned solution: not yet researched. Candidate: replace `exclude.contains` with a
`HashSet` of excluded ids (O(1) membership). R is small in practice.

Status: identified, not yet researched.

## Open decisions for the user (not settled by any doc)

These are genuine tradeoffs. `decisions.md` recorded one side but is **not binding**
— the user decides. (Note: `decisions.md` has been removed after salvaging the valid
parts into this file.)

- **Keep `total` vs cursor.** Keeping `total` costs an O(A) full scan on agdb *list*
  endpoints; the search-page `total` is free (SeekStorm, no agdb scan). Dropping it
  for cursor pagination is viable only if jump-to-page is not a product need.
- **Master-doc-per-article (P1) vs version/comment-indexed.** The current design is
  version+comment granular (`one doc per version + one per comment`), which keeps
  highlight cost O(single field) but makes article paging cost O(offset). Master-doc
  reverses it: cheap article paging, but loses per-version/comment highlight and risks
  article-level bloat.
- **No ORDER BY for search.** Only helps the empty-query/browse path
  (`search_iterator_index`, `search.rs:1413-1432`); keyword search still ranks by BM25.
  Removing the user-facing `sort` controls is a product decision, not a perf win.
- **No ORDER BY for listing (P5).** Verified `LimitOffsetHandler` short-circuit, but it
  drops newest-first ordering — see P5.

## Decision status

| Item | Status |
| --- | --- |
| P1 deep pagination (master doc) | Proposed — conflicts with version/comment-indexed design |
| P2 enrich_articles localize (`where_.ids`) | Proposed (salvaged from decisions.md) |
| P3 enrich_comment_headers batching | Proposed |
| P4 sync_all | Accepted as inherent |
| P5 pagination sort + slice | Identified; drop ordering → LimitOffsetHandler, pending UX call |
| P6 recycler O(R²) | Identified, pending research |
| Search: no ORDER BY / cursor / no total | Open — user decides |

_Last updated: after research session on library sources (seekstorm 3.3.5, agdb 0.13.2)._