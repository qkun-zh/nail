# ADR-0002: Search Frontend-Backend Mismatch Report

## Status
Analysis complete. No decision made yet. Fixes pending owner review.

## Context
The new Rust workspace (`nail`) reimplements search across three layers:
common data contracts (`code/common`), backend axum handlers (`code/back`),
and Leptos CSR frontend (`code/front`). This report compares the two sides
end-to-end via source reading + backend test probes. Legacy code at
`document/legacy/` was consulted as reference but is NOT the target.

The comparison covers: dispatch logic, query-param serialization, response
contract deserialization, pagination, time filters, snippet/highlight
rendering, and server-config enforcement.

## Scope of comparison

| Layer | File |
| --- | --- |
| Common contract | `code/common/src/search.rs` (enums), `code/common/src/request.rs` (`ArticleSearchParams`) |
| Backend dispatch | `code/back/src/logic/search.rs`, `code/back/src/logic/article.rs` |
| Backend repository | `code/back/src/repository/search.rs`, `code/back/src/repository/search/document.rs` |
| Backend interface | `code/back/src/interface/article.rs`, `code/back/src/interface/extractor.rs` |
| Frontend request | `code/front/src/request/article.rs`, `code/front/src/request/url.rs` |
| Frontend page | `code/front/src/page/public/article/search.rs` |
| Frontend pagination | `code/front/src/page/pagination.rs` |
| Config | `configuration/server.toml`, `code/back/src/infrastructure/config/server.rs`, `code/front/src/infrastructure/limits.rs` |

## Verified: dispatch logic always agrees

The frontend decides "search vs list" in `is_search_active()` by checking
whether **any** of `{q, ranges, sort, from, to}` is non-empty. The backend
decides in `is_search_request()` (same four fields). Because both sides use the
identical condition, dispatch is always consistent:

- `q` non-empty → both enter search mode → `SearchPage` deserialization ✓
- `sort` non-empty only → both enter search mode → backend `enable_empty_query = true` ✓
- No params → both enter list mode → `ArticleListPage` deserialization ✓

No 400-error scenarios arise from dispatch disagreement.

## Verified: query-param serialization is consistent

| Param | Frontend encoding | Backend decoding | Status |
| --- | --- | --- | --- |
| `q` | URL-encoded, percent-encoding via `url` crate | `ArticleSearchParams.q: Option<String>` | ✓ |
| `ranges` | Repeated `ranges=` keys, comma-separated enum names | `Option<String>` parsed by `parse_ranges` | ✓ |
| `sort` | `field:direction` (e.g. `time:desc`) | `ArticleSortParams.parse()` → `(SearchSortField, SearchSortDirection)` | ✓ |
| `from`/`to` | Epoch seconds (i64) via `datetime_local_to_epoch_secs` | `Option<i64>` used in SeekStorm `FacetFilter::Timestamp` comparison | ✓ |
| `page` | Integer, starts at 1 | Clamped by `clamp_page_limit` (1..=10000) | ✓ |
| `limit` | `search_page_size` (default 8) | Clamped by `clamp_page_limit` (1..=200) | ✓ |

All enum `rename_all = "lowercase"` settings on `SearchRange`,
`SearchSortField`, and `SearchSortDirection` produce the exact string keys
(`title`, `summary`, `author`, `comment`, `note`, `tag`, `time`, `asc`, `des`)
that both sides expect. Tests in `test/unit/common/search/tests.rs` confirm
serialization round-trips.

## Verified: backend test suite passes

```
cargo test --bin nail_back -- search   →  14 passed, 0 failed
cargo test --bin nail_back -- article  →  all passed
cargo test --bin nail_back             →  305 passed, 0 failed
```

## Mismatches found

### MISMATCH 1: Query length validation — bytes vs characters

**Severity:** Medium (correctness: config intent vs enforcement)

**Location:** `code/back/src/logic/search.rs:21`

**Description:**
The config field is named `max_search_query_chars` (512), the error message
says "chars", and `configuration/server.toml:21` documents it as characters.
Yet the check uses `trimmed.len()` (byte count), not `trimmed.chars().count()`:

```rust
// logic/search.rs:21
if trimmed.len() as u64 > max_query_chars {
    return Err(LogicError::SearchQueryTooLong);
}
// error message (line 23):
"search query too long (max {max_query_chars} chars)"
```

For ASCII input, 513 bytes = 513 chars → rejected (correct behavior).
For multi-byte UTF-8 (e.g., CJK, emoji), the effective character limit is far
below 512: 512 bytes ≈ 170 Chinese characters. A user typing 512 CJK characters
(1536 bytes) is rejected with "max 512 chars" despite being within the stated
limit.

**Legacy comparison:** `document/legacy/code/back/src/logic/article_search.rs:42`
uses `trimmed.chars().count()`, correctly enforcing the character limit.

**Fix recommendation:**
```rust
if trimmed.chars().count() as u64 > max_query_chars {
```

**Test gap:** `test/unit/back/logic/search.rs` only tests ASCII (`"a".repeat(513)`),
which passes both byte-based and char-based checks. Add a probe with multi-byte
characters at the boundary.

---

### MISMATCH 2: Double search-result snippet highlighting

**Severity:** High (visual defect: literal `<mark>` / `</mark>` text visible)

**Location:** `code/front/src/page/public/article/search.rs:135-141`
(combined with `code/back/src/repository/search.rs` highlighter)

**Description:**
The backend already runs SeekStorm's `highlighter` (via `repository/search.rs`)
on the indexed `title`/`summary`/`author`/`comment`/`note`/`tag` fields,
wrapping matched terms in `<mark>…</mark>` tags before returning `SearchHit.snippet`.

The frontend then calls `render_snippet(&hit.snippet, &terms)` which:
1. Calls `escape_html()` on the already-HTML snippet → turns `<mark>` into
   `&lt;mark&gt;` (visible literal text).
2. Calls `highlight_terms()` → re-applies new `<mark>` tags around the terms.

Result visible to user:
```
…&lt;mark&gt;<mark class="...">searchterm</mark>&lt;/mark&gt;…
```
When rendered via `inner_html`, the browser displays literal `<mark>` and
`</mark>` surrounding the highlighted term.

**Code trace:**
```rust
// front/src/page/public/article/search.rs
let snippet_html = render_snippet(&hit.snippet, &terms);
// renders: inner_html = snippet_html
//
// render_snippet does:
escape_html(snippet).then(|escaped| highlight_terms(&escaped, terms))
//         ^^^^^^^ destroys backend <mark> tags
```

**Legacy comparison:** Legacy code at `document/legacy/code/front/src/page/search.rs`
did not have a `render_snippet` that double-escaped; legacy snippets were
trusted from the backend.

**Fix recommendation (Option A — simplest, recommended):**
Remove `render_snippet` from the hit rendering and use the backend snippet
directly:
```rust
<span inner_html=hit.snippet></span>
```
The backend's highlighter already wraps terms in `<mark>` tags, so no frontend
re-highlighting is needed.

**Alternative (Option B):** If frontend-side highlighting must be retained
(e.g., for consistency with unhighlighted fields), strip existing `<mark>` tags
and unescape before re-highlighting — but this is redundant since the backend
already does the work.

**Test gap:** No frontend test covers `render_snippet` or `highlight_terms`.
No backend integration test verifies the combined snippet-highlight flow.

---

### MISMATCH 3: `total_pages` not capped in list mode

**Severity:** Low (pagination display inconsistency, not a crash)

**Location:**
- Search path (capped): `code/back/src/logic/search.rs:67-69`
- List path (uncapped): `code/back/src/logic/article.rs:161-163`

**Description:**
The backend caps `total_pages` at `max_search_pages` (1024) in the search path:
```rust
// logic/search.rs:67-69
let raw_total_pages = outcome.total.div_ceil(limit);
let total_pages = raw_total_pages.min(state.config.server.max_search_pages);
let truncated = raw_total_pages > state.config.server.max_search_pages;
```

But the list path does NOT cap:
```rust
// logic/article.rs:161-163
let total_pages = total.div_ceil(limit);
let truncated = total_pages > state.config.server.max_search_pages;
```

`total_pages` in list mode can be very large (e.g., 10,000 for 80,000 articles
at page_size=8). The frontend's `Pagination` component renders the full
`total_pages` value in its `max="…"` attribute and page counter. In search mode,
the frontend receives a capped `total_pages` of 1024.

This means the pagination UI behaves differently depending on whether the user
is in search vs list mode, even though the same `Pagination` component is used.

**Legacy comparison:** Legacy `document/legacy/code/back/src/logic/article_search.rs:47`
caps in the search path; list path at `document/legacy/code/back/src/logic/article.rs`
does not cap — same pattern.

**Fix recommendation:** Apply the same `min(max_search_pages)` cap in the list
path:
```rust
let total_pages = total.div_ceil(limit).min(state.config.server.max_search_pages);
```

---

### MISMATCH 4: `include_uncommitted = true` in search document reads

**Severity:** Low (read-committed vs read-uncommitted visibility)

**Location:** `code/back/src/repository/search.rs:268`

**Description:**
When fetching a SeekStorm document for search-hit snippet extraction, the
backend passes `include_uncommitted = true`:
```rust
// repository/search.rs:268
let document = self
    .index
    .get_document(hit.doc_id, true, ...)  // true = include uncommitted
```

The legacy code at
`document/legacy/code/back/src/repo/search.rs:260` passed `false`:
```rust
let document = index.get_document(hit.doc_id, false, ...)
```

This means the new backend may return snippets from documents that have not
been fully committed to the SeekStorm index, potentially showing stale,
partial, or inconsistent data in search results.

**Fix recommendation:** Change `true` to `false` to match legacy behavior
(read-committed semantics):
```rust
.get_document(hit.doc_id, false, ...)
```

---

### MISMATCH 5: Timezone mismatch in `from`/`to` time filters (noted)

**Severity:** Out of scope — this is a design issue, not a new-code regression

The frontend's `datetime_local_to_epoch_secs` uses the **browser's local
timezone** to convert `datetime-local` input to epoch seconds. The backend
formats and displays article times using the server's
`timezone_offset_seconds` (config: 28,800 = UTC+8). If a user in a different
timezone uses the time-range filter, the intent-to-epoch conversion diverges
from the displayed server timezone.

This affects both the new and legacy code equally. See `README.md §8.1` for
the "verify every legacy line individually" guidance, but this is a
pre-existing design gap rather than a frontend/backend contract mismatch in
the new code.

---

## Summary table

| # | Mismatch | Severity | Frontend file | Backend file |
| --- | --- | --- | --- | --- |
| 1 | Query length: bytes vs chars | Medium | — | `logic/search.rs:21` |
| 2 | Double snippet highlighting | High | `page/.../search.rs:135` | `repository/search.rs` (highlighter) |
| 3 | `total_pages` not capped in list mode | Low | (receives value) | `logic/article.rs:161` |
| 4 | `include_uncommitted = true` | Low | — | `repository/search.rs:268` |
| 5 | Timezone in time filters | N/A (pre-existing) | `request/url.rs` (datetime conversion) | (config-driven) |

## Test coverage gaps

| Gap | File | Action |
| --- | --- | --- |
| Query length only tested with ASCII | `test/unit/back/logic/search.rs` | Add multi-byte probe at 512-char boundary |
| No frontend test for `render_snippet` / `highlight_terms` | `test/unit/front/` | Add snapshot test for snippet HTML output |
| No test verifying list-mode `total_pages` capping | `test/unit/back/logic/search.rs` | Add test that list path caps at `max_search_pages` |
| No test for `include_uncommitted` flag | `test/unit/back/repository/search.rs` | Add test that uncommitted docs are not returned |

## Backend test commands

```
cargo test --bin nail_back -- search
cargo test --bin nail_back -- article
cargo test --bin nail_back
```
All pass currently (305 tests, 0 failures).
