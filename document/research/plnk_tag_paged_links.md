# Research: generic tailwind paginator + tag page wiring

**Code:** `plnk`

## 1. Requirement

`R₀`: Build a generic, Tailwind-utility-styled paginator component
(prev / jump-input / next, URL-driven via `?page=N`), isolated from the search
page's pagination machinery, and apply it first to the `/tag` list page. Tag
list pagination uses its own config-driven page size, not `search_page_size`.

User decisions (§3): (1) URL-driven; (2) do NOT reuse `search_page_size` —
new config key; (3) controls = prev + jump page input + next; (4) generic
paginator component (pure UI), list rendering supplied by the consuming page.

## 2. Research questions

- U1: Does the backend already paginate tag reads?
- U2: How is a URL-driven page parameter managed in the client?
- U3: What code touches `search_page_size` / `RuntimeLimits`, i.e. the full
  path for adding a new page-size config key?
- U4: Can tags-bound pagination reuse the existing `AppPaged` extractor and
  `clamp_page_limit` without entangling search?

## 3. Evidence

### U1 — tag reads are already paginated server-side (source)

- `code/server/src/logic/tag.rs:32` `read_tags(state, actor, page, limit)`
  → `paginate(tags, page, limit)` → `ListPage { items, has_next, total }`
  (`code/server/src/logic/pagination.rs:10` `paginate`).
- `code/server/src/interface/tag.rs:27` route `read_tags` extracts
  `AppPaged((page, limit))`.
- `code/server/src/interface/extractor.rs:77` `AppPaged`: defaults page=1,
  limit=`search_page_size`, guards `page > max_search_pages` (bad request).
- http test already calls `/tags?page=1&limit=200`
  (`code/server/src/tests/http/tag_apply.rs:38`).

**Probe:** `server/src/tests/probe_012_tag_paged_reads.rs` (see §below) —
asserts explicit page/limit behavior over 25 tags. Runs green on baseline.

### U2 — URL-driven query signal (source)

- `leptos_router` 0.8.15 (`code/client/Cargo.toml:13`), source
  `~/.cargo/registry/.../leptos_router-0.8.15/src/hooks.rs:84-146`:
  `query_signal<T>` / `query_signal_with_options<T>` exist;
  `T: FromStr + ToString + PartialEq + Send + Sync`; setter navigates with
  given `NavigateOptions` (e.g. `replace: true`) preserving other params.
- Client precedent: `code/client/src/page/article/search.rs:198-221` uses
  `query_signal_with_options` for `q`, `ranges`, `from`, `to`, `page`.
- 404 risks if called outside `<Router>` — component must be used under
  routes (tag page is).

### U3 — full path for a new runtime page-size config key (source)

Adding `tag_page_size` touches, in dependency order:

1. `code/common/src/response.rs:41` `RuntimeLimits` (wire format; client
   deserializes it from `/config`).
2. `code/server/src/infrastructure/config/server.rs:4-22` `ServerConfig`
   fields + `validate` loop `:61-72`.
3. `configuration/server.toml` (source of values).
4. `code/server/src/interface/config.rs:12-22` `/config` response build.
5. `code/server/src/tests/context.rs:301-319` `test_config()` fixture
   (must add fields or `TestCtx` fails to build).
6. `code/client/src/infrastructure/limits.rs:4-48` compile-time defaults +
   `apply_fallbacks`.
7. Tests: `code/server/src/tests/configuration/validation.rs:68-69,156-157`
   (invalid + valid toml fixtures), `code/server/src/tests/http/config.rs:21-22`
   (keys asserted on wire), `code/client/src/infrastructure/limits_tests.rs`.

### U4 — reusing `AppPaged` / `clamp_page_limit` without entanglement

- `AppPaged` hardcodes `search_page_size`/`max_search_pages` defaults
  (`extractor.rs:87-92`) and is used by tag, user, role, version, comment
  routes. Changing its defaults would reshape every collection → out of scope.
- Tag route instead uses `AppQuery<PagedQueryParams>` + a new pure helper
  `clamp_page(page, limit, default_limit)` (no max-page guard). `AppPaged`,
  `clamp_page_limit`, and every search-facing message stay byte-identical —
  zero lines of search code change.
- Far-page safety: `paginate` only `skip`s an already-loaded Vec
  (`pagination.rs:10-17`); existing `probe_review_findings.rs:31-43` asserts
  `read_tags(page=2, limit=200)` returns an empty page without panicking.
  No `max_page` guard is therefore needed.
- `PagedQueryParams` (`extractor.rs:69-73`) is currently `private` → must
  become `pub` for the tag route to reuse it.

### U5 — client pagination UI that must NOT be reused (source)

- `code/client/src/page/pagination.rs` holds search-facing parts:
  `LevelPagination`, `PrevNext`, `Pagination`, `LocalPagedList`. The tag
  paginator is a NEW module (`paged_links.rs`) with its own helper math, so a
  future search refactor cannot touch it.
- Styling tokens available: `tailwind.css` `@theme` colors
  (`--color-ink/muted/faint/line/line-strong/card`) + `--font-mono`; Tailwind
  v4 already scans `.rs` files (precedent `delete_mode.rs:24` uses
  `mt-4 flex w-auto flex-wrap items-center gap-2`).

## 4. Findings

- Backend pagination exists; only page-size *defaults* are search-bound.
- The user-anticipated "backend cooperation" is a small, well-scoped config
  key addition.
- Far pages are safe (empty list, no error), so no maximum-page guard is
  built — user explicitly declined one.
- The whole feature is 3 commits: server config, client infra, client UI.

## 5. Impact on R

`R₀` stands, refined to `R` (see exec doc):

- Share no code with `page/pagination.rs`; standalone `paged_links.rs`.
- New config key `tag_page_size` (default 8); no maximum-page limit.
- `PagedLinks<T>` renders context bar + item list (consumer-supplied) +
  prev / jump input / next; URL query `?page=N` drives fetch.

## 6. Open items

- None blocking. Cosmetic choices (context-bar copy, exact Tailwind classes)
  are plan-level and visible in the browser at slice 3.

## Probe

`server/src/tests/probe_012_tag_paged_reads.rs` (registered in `harness.rs`):
25 tags created via API; `/tags?page=1&limit=8` → 8 items, total=25,
has_next=true; `page=2` → 8 items has_next=true; `page=4` → 1 item
has_next=false; `page=3&limit=10` slice check. Green on baseline
(explicit params, future-proof).