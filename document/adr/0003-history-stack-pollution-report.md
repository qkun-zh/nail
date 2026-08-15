# ADR-0003: Frontend History-Stack Pollution Report

## Status

Fixed. Both pollution sites corrected with `replace: true`. Frontend compiles
(`cargo check` + `cargo test`: 66 passed, 0 failed) and `cargo clippy` reports
no new warnings from these changes.

## Summary

Two programmatic `navigate()` calls omit `replace: true`, so they call
`history.pushState` — each pagination click pushes a new entry onto the browser
history stack. For a frontend whose back-navigation depends entirely on the
history stack, this turns one logical "visit" into N stack entries (one per page
number clicked), forcing N+ back-button presses to exit the page.

All other `navigate()` call sites in the codebase correctly set `replace: true`
(or are ordinary `<A>` link clicks, which push by design).

## How `replace` works in Leptos Router 0.8.15

Verified from library source at
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/leptos_router-0.8.15/`:

**`NavigateOptions` default** (`navigate.rs:19-27`):

```rust
impl Default for NavigateOptions {
    fn default() -> Self {
        Self {
            resolve: true,
            replace: false,   // ← defaults to FALSE → pushes a history entry
            scroll: true,
            state: State::new(None),
        }
    }
}
```

**`BrowserUrl::complete_navigation`** (`location/history.rs:214-251`):

```rust
fn complete_navigation(&self, loc: &LocationChange) {
    let history = window().history().unwrap();
    let current_path = self.path_stack...last().map(|url| url.to_full_path());
    let add_to_stack = current_path.as_ref() != Some(&loc.value);

    if loc.replace {
        history.replace_state_with_url(&loc.state.to_js_value(), "", Some(&loc.value)).unwrap();
    } else if add_to_stack {
        // push the "forward direction" marker
        history.push_state_with_url(state, "", Some(&loc.value)).unwrap();  // ← PUSHES
    }
    ...
}
```

When `replace` is `false` and the new URL differs from the current one
(`add_to_stack == true`), the router calls
`history.push_state_with_url` — a **new entry** on the browser history stack.
When `replace` is `true`, it calls `history.replace_state_with_url` instead,
which replaces the current entry and leaves the stack depth unchanged.

## Pollution Site 1 — Version list pagination

**File:** `code/front/src/page/public/article/version/index.rs:80-87`

```rust
let navigate = navigate.clone();
let on_go = Callback::new(move |target: u64| {
    navigate(
        &format!("/public/article/{article_id}/version?page={target}"),
        NavigateOptions {
            resolve: false,
            ..Default::default()   // ← replace defaults to false → PUSHES
        },
    );
});
```

**Trigger:** Clicking "prev"/"next"/entering a page number in the version-list
`Pagination` component (`page/pagination.rs`), which calls `on_go.run(target)`.

**Impact:** Each page turn pushes a new `?page=N` entry. A user who pages
through versions 1→2→3→4 will have four extra history entries. Pressing
"back" will step through page 3, page 2, page 1, then the article detail —
rather than jumping straight back to the article detail in one press.

**Fix applied:** Added `replace: true` (now at `index.rs:85`).

## Pollution Site 2 — Comment pagination

**File:** `code/front/src/page/public/article/version/comment/pagination.rs:23-31`

```rust
move |page: u64| {
    navigate(
        &format!("{base_href}?page={page}"),
        NavigateOptions {
            resolve: false,
            ..Default::default()   // ← replace defaults to false → PUSHES
        },
    );
}
```

**Trigger:** Clicking "prev"/"next"/entering a page number in the
`LevelPagination` component (same `Pagination` base), which calls
`on_go.run(page)`.

**Impact:** Each comment page turn pushes a new history entry. Because comment
pagination is nested inside version-detail → comment-thread, the pollution
compounds: reading through 5 comment pages adds 5 stack entries, requiring 5+
back presses to exit the comment thread.

**Fix applied:** Added `replace: true` (now at `pagination.rs:28`).

## Already correct — `replace: true` everywhere it matters

These handlers explicitly set `replace: true` and do **not** pollute the stack:

| File | Line | Purpose |
| --- | --- | --- |
| `page/draft.rs:42-46` | `persist_draft` | Sync draft form fields into query string |
| `page/public/article/search.rs:259-266` | `sync_url` | Sync search query params into URL |
| `page/public/article/version/index.rs:48-55` | `sync_navigate` | Sync version-list `?page=N` into URL |
| `page/public/article/version/comment.rs:73-80` | `sync_url` | Sync comment `?page=N` into URL |
| `page/public/article/version/comment.rs:274-281` | delete handler | Navigate away after comment delete |

The `persist_draft` helper (`page/draft.rs`) is the shared utility used by
`authenticate`, `deregister`, `name/update`, `email/update`, `article/create`,
`article/update`, `version/create` — all via `replace: true`. This is correct.

The `sync_url`/`sync_navigate` Effects in `search.rs`, `comment.rs`, and
`version/index.rs` also use `replace: true` — correct pattern. The two
pagination `on_go` callbacks are the only deviations from this pattern.

## Non-issues (verified not to pollute)

### `<A>` link components — push by design
All `<A>` components in the codebase (e.g. `comment.rs:209`,
`version/index.rs:75`, `search.rs:655`, `detail.rs:70`, `index.rs:7`,
`session_gate.rs:74`) use default `replace: false`. This is **expected** —
clicking a link to navigate to a different page should add to the history stack
(standard browser behavior). No `<A>` uses `prop:replace`; none should, since
they all represent genuine page-to-page navigation via page links.

### `query_signal` setters unused
`query_signal::<u64>("page")` is called in `version/index.rs:25` and
`comment.rs:32`, and the setter is bound to `_set_page` (underscore prefix =
intentionally unused). The `query_signal` setter internally calls `navigate()`
with `NavigateOptions::default()` (`replace: false`), so if it were ever called
it would push. Currently it is never called — page state is instead synced via
the `replace: true` `sync_url`/`sync_navigate` Effects. This is a latent risk:
if `_set_page` is ever wired up, it will pollute the stack. It should be
documented or replaced with a read-only query parser.

### Download flows
- `download.rs:save_blob` (`download.rs:106-129`) creates a temporary `<a>`,
  sets `href` to an object URL, sets the `download` attribute, and calls
  `click()`. The `download` attribute makes the browser perform a download, not
  a navigation — no history entry is added.
- `version/detail.rs:DownloadLink` (`detail.rs:12-32`) attaches an `on:click`
  handler that calls `event.prevent_default()` before performing an async
  download — the default navigation is cancelled.
- `download.rs:69` reads `window.location().origin()` (read-only, no mutation).

### No global history mutations
No calls to `window.history.back()`, `window.history.forward()`,
`window.history.go()`, `window.location.assign()`, `window.location.replace()`,
`location.href = ...`, or `window.open()` exist anywhere in the frontend source.

### No proxy-level redirects
Pingap configuration (`configuration/proxy/`) has no redirect rules. The
`/api` location reverse-parks to the backend; the `/` location serves the
SPA `index.html` with a rewrite fallback. No 301/302 responses that would add
to the user's history stack.

## Root-cause pattern

The codebase has two idioms for updating the URL from reactive state:

1. **`sync_url` / `sync_navigate` / `persist_draft`** — an `Effect` that watches
   form/query signals and replaces the URL with `replace: true`. Used for
   persisting draft form state and syncing filter/page params. **Correct.**

2. **Pagination `on_go` callback** — fires from a user button click to change the
   page query param. Uses `Replace: false` (default). **Incorrect** — pagination
   is a UI state change, not a navigation to a new conceptual page; it should
   replace, not push.

The two pollution sites fell into the wrong idiom: they look like
`sync_url` but omitted `replace: true`.
