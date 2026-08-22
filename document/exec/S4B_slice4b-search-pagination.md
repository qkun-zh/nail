# Exec S4B slice4b-search-pagination

## Requirement
Remove second paginate in logic/search.rs:search_articles; use searcher offset/limit directly.

## Scope
In: `code/server/src/logic/search.rs` — fix search double pagination only.
Out: read_users/roles/tags, AppPaged, clamp duplication.

## Design
- Keep clamp_page_limit single call, keep offset calc.
- Request limit+1 from searcher to detect has_next; assemble tree; if assembled > limit truncate and has_next true.
- total = assembled len before truncate? To keep ListPage total plausible, use items.len() + if has_next ? 1 : 0 or items.len() — choose items.len() as page total (net reduction, still ListPage shape).
- Simpler without limit+1: just use article_list as items, has_next = items.len() as u64 == limit and page < max; total = items.len(). Choose limit+1 for correctness.

## Slices
1. Search pagination fix — files: logic/search.rs, infrastructure/search.rs (pass through limit+1), pagination import removed if unused.

## Verification
- cargo fmt, cargo clippy -p server
- cargo test -j1 -p server

## Risks
Searcher groups docs to articles, so limit+1 on docs != articles+1; has_next may be approximate but removes double offset bug.
