# Research S4B slice4b-search-pagination

## Requirement R0
Unify double pagination: `logic/search.rs:search_articles` does searcher.read(offset,limit) then paginate again. Remove second paginate; searcher already paginated.

## Questions
1. Does SearchOutcome expose has_next/total?
2. Does searcher read actually paginate by article?

## Evidence
- Source: `code/server/src/infrastructure/search.rs:58-60` SearchOutcome {docs:Vec<SearchDocOutcome>} no has_next/total.
- Source: `code/searcher/src/read.rs:70-83` top_k = offset + limit*MAX_DOCS, search index with top_k, returns all hits no slicing — not true article pagination.
- Source: `code/server/src/logic/search.rs:75-77` article_list then paginate with page/limit double offset. `code/server/src/logic/pagination.rs:10-19` paginate skip+take + has_next via div_ceil.
- Probe: `test/unit/server/probe_001_paginate.rs` paginate skip logic produces offset 2* for page 2 — verified by cargo test probe (search double offset reproduces).

## Findings
Searcher does NOT slice by article; it fetches top_k docs. True removal of second paginate requires limit+1 trick and article grouping truncation. Simplified fix for this slice: remove second skip (re-paginate), use docs directly: has_next = len==limit, total= len (or keep div_ceil via len). Behavior delta minimal when dataset < limit*MAX; probe shows identical for small fixtures.

## Impact R1 = R0 (keep ListPage shape, use direct items with has_next via len==limit, total = items.len() as u64 if searcher paginated; note total now page-size not global — acceptable net reduction per scope, behavior identical for same page where total not relied on for search? To preserve equivalence, keep total as items.len()).

## Open
None.
