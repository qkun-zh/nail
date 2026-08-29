# Exec: generic tailwind paginator + tag page wiring

**Code:** `plnk` | Research: `research/plnk_tag_paged_links.md`

## 1. Requirement

`R`: Deliver a generic, Tailwind-utility-styled paginator that is URL-driven
(`?page=N`), fully isolated from the search page's pagination machinery, and
wire the `/tag` list page to it. Tag pagination uses a new config-owned page
size (`tag_page_size` / `max_tag_pages`), never `search_page_size`.

Acceptance criteria:
- `/tag` shows a Tailwind-styled link list: context bar (total count) +
  rows (tag -> detail link, article count) + controls `[prev] [input / N]
  [next]`.
- Changing page (input enter/arrows, prev/next, browser back/forward)
  updates `?page=N` and refetches; deep link `/tag?page=2` lands on page 2.
- Tag default page size comes from runtime config, distinct key from search.
  No maximum page-number limit (far pages yield empty lists, validated).
- No code shared with `page/pagination.rs` (search-facing) or
  `page/article/search/**`.

## 2. Scope

- **In:** server+common config key `tag_page_size`; `PagedQueryParams`
  exposure; new `clamp_page` helper (no max-page guard); tag route custom
  defaults; client limits; generic `PagedLinks<T>` component + tests; tag
  page wiring.
- **Out:** role/user/version/comment page_size keys (stay on current
  defaults); search page refactor (zero lines of search code change);
  cosmetic list styling beyond a readable warm-palette link list.

## 3. Design decisions

- **D1** New runtime key `tag_page_size=8` in `ServerConfig` +
  `RuntimeLimits`, sourced from `configuration/server.toml`. No
  `max_tag_pages`: far pages are safe because `paginate` only `skip`s an
  already-loaded Vec (probe_012 + existing `probe_review_findings` prove
  far-page behavior returns an empty page without panicking).
- **D2** Tag `read_tags` interface route drops `AppPaged` for
  `AppQuery<PagedQueryParams>` (made `pub`) + new pure helper
  `clamp_page(page, limit, default_limit) -> (page≥1, limit∈[1,MAX])`
  in `logic/pagination.rs`. `AppPaged`, `clamp_page_limit`, search callers
  and every search message test stay byte-identical → search fully untouched.
- **D3** New standalone module `client/src/page/paged_links.rs` owns its own
  page-math helpers (`total_pages`, `parse_page_param`, `clamp_page`);
  nothing imported from `pagination.rs`.
- **D4** URL param `page` via `query_signal_with_options::<u64>("page",
  replace)` (leptos_router 0.8.15 hooks, source-verified); resource keyed to
  the param refetches on URL change; browser back/forward works for free.

## 4. Slices

### S1 — server + common: tag page_size config
- **Goal:** tag list reads default to `tag_page_size` (no max-page guard),
  key exposed on `/config`.
- **Files:**
  - `code/common/src/response.rs` (RuntimeLimits +1 field)
  - `code/server/src/infrastructure/config/server.rs` (field + validate)
  - `configuration/server.toml`
  - `code/server/src/interface/config.rs`
  - `code/server/src/logic/pagination.rs` (new `clamp_page`, no limits change)
  - `code/server/src/interface/extractor.rs` (PagedQueryParams pub)
  - `code/server/src/interface/tag.rs` (own defaults via AppQuery)
  - tests: `tests/context.rs`, `tests/configuration/validation.rs`,
    `tests/http/config.rs`, `tests/http/tag_apply.rs` (default-limit cases)
- **Red:** new tests reference the field/prop → compile-red until impl.
- **Green:** `/config` exposes `tag_page_size`; `/tags` no-params returns
  `tag_page_size` items with `total` intact; `?page=9999` returns an empty
  page, not an error; all existing search-message tests unchanged & green.
- **Exit:** `cargo test -j 1 -p common`; `cargo test -j 1 -p server`
  (incl. probe_012); `cargo clippy -p server`; `cargo fmt`.
- **Commit:** one commit, clean tree.

> **Merge note (S1+S2):** `RuntimeLimits` is shared wire format; adding a
> field breaks the client's `compile_time_defaults`/`apply_fallbacks`
> constructor until S2 lands. S1 and S2 are therefore committed together as
> one slice so the tree stays CI-green. Client infra verified here
> (`limits_tests` 4/4).

### S2 — client infra: limits
- **Goal:** `use_limits().tag_page_size` available client-side.
- **Files:** `client/src/infrastructure/limits.rs`,
  `client/src/infrastructure/limits_tests.rs`.
- **Red/Green:** defaults + fallbacks asserted (compile + cases).
- **Exit:** `cargo test -j 1 -p client`; `cargo clippy -p client`; fmt.

### S3 — client UI: PagedLinks + tag page
- **Goal:** generic `PagedLinks<T>` and wired `/tag`.
- **Files:**
  - `client/src/page/paged_links.rs` (new) + `paged_links_tests.rs` (new)
  - `client/src/page.rs` (mod registration)
  - `client/src/page/tag/list.rs` (rewrite)
- **Green:** math helpers unit-tested; tag list renders via component;
  `?page=2` refetches; empty state text; error path shows message.
- **Exit:** client tests; `cargo clippy -p client`; `trunk build`; browser
  smoke: `/tag`, jump input, prev/next, deep-link, back/forward.
- **Commit:** one commit, clean tree.

## 5. Open unknowns

- None. `query_signal` behavior source-verified; browser confirmation is S3
  smoke (manual).

## 6. Verification plan

Per slice: exit commands above (fmt + clippy zero-warning + tests). S3 adds
`trunk build` + manual browser walk. CI gate per §9 (push + watch).

## 7. Risks

- Tag count in dev DB may be < page size → pagination invisible. Mitigate:
  smoke with existing seeded tags; if fewer than 8, verify via `?limit=1`
  deep link and seed if owner wants (ask, don't self-seed data).
- `query_signal` setter uses requestAnimationFrame batching — a second
  rapid page-hop could coalesce; acceptable (replace semantics).

## 8. Constraints

- English UI copy only. No hardcoded limits (toml + RuntimeLimits source).
- Panic-free; no `unwrap`/`expect`. No edits to search modules.
- `cargo clippy` plain (tests exempt), `cargo fmt` clean.
- One commit per slice; never discard work.

## 9. Questions

- None blocking pending §8 adoption.