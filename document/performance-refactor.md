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

**Approved? NO — OPEN.** User has not approved P6. No observable behavior change (the
recycler chosen is unchanged); no risk; reaches the theoretical optimum (O(R) instead of
O(R²)) — but it awaits explicit user approval before implementation.

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
| P3 enrich_comment_headers batching | **Done** (commit cf701c4) |
| P4 sync_all | Accepted as inherent (no change) |
| P5 pagination sort + slice | **Open** — drops newest-first ordering |
| P6 recycler O(R²) → HashSet | **Open** — not approved |
| Search: no ORDER BY / cursor / no total | **Open** — user decides |

_Last updated: P2 and P3 both implemented (20fdeb4, cf701c4) and removed from pending
tracking. P6 marked **not approved** (open). P1/P5/total/cursor open. Baseline: build
green, 306 tests pass._

## Probe evidence log

| # | Probe | Result |
| --- | --- | --- |
| P3 | `probe.rs::probe_batch_comment_enrichment_matches_per_comment` | Passed. 4 comment pairs (2 distinct articles, 2 versions, 1 author; incl. duplicate + cross pair): batch path returns identical (title, author, version) to per-comment. |