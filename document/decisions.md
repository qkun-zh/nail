# Decisions

Read-only record of the architecture decisions and conventions the code follows.
To change one, re-evaluate it explicitly (add a note here) rather than silently
diverging. Grouped by area; each entry states the outcome and its key consequence.

## Authorization and deletion

- **Delete is split by mode as opaque Cedar actions.** Every resource gets
  `Delete::{Hard, Transfer, Soft}` (some variants future-facing). Cedar action
  names have no hierarchy: granting `Hard` does not grant `Transfer`; each is
  seeded and granted explicitly.
- **Authorization lives entirely in Cedar.** Backend logic does plain CRUD and
  calls `authorize`/`authorize_or` with a specific action; it never re-implements
  "may X do A on R". Business invariants (role immutability, duplicate content,
  version ordering, transfer ownership) stay in the backend — they are about the
  resource's own state, not the actor.
- **Owners soft-delete only; hard delete is admin-only.** Published content is a
  platform asset. `Hard` delete is admin-only (admin holds all `Hard`).
- **Recycler is a terminal sink.** Principal-side Cedar `forbid` stops a recycler
  from transferring anything; recycler-owned content can never be transferred.
  Grant matrix: admin = all Hard + Transfer; member = Article/Comment Transfer;
  recycler = none. Note: `user_zero` is both admin and recycler, so admin loses
  transfer/soft-delete (accepted).
- **Interface verbs are a closed set** `create/read/update/delete`; `read` is
  never a side-effecting flow, `intent`-style wire vocabulary never appears as a
  backend identifier, and route/handler/logic verbs must agree. Search is a
  `read` with query parameters, not a fifth verb. (`/email/read`→`/token/create`
  with `purpose`; `/challenge/read`→`/challenge/create`.)
- **The content/PDF read path is Cedar-gated** (`PERMISSION_VERSION_READ`); the
  backend no longer exposes an `is_author` ownership flag — the client compares
  `author_id` to the session user instead.

## Search

- **Version-indexed search (implemented).** One SeekStorm index, one document
  per version + one per comment, distinguished by a `doc_type` facet. Version
  fields: `title`, `summary`, `author_name`, `note`, `tags`, `version_number`,
  `ts`. Comment fields: `author_name`, `content`, `ts`. Keeps highlight cost
  O(single field) and avoids article-level comment bloat.
- **The response is a tree, not flat hits.** `SearchPage → article →
  version → comment`, carrying `id`/`version_id`/`comment_id` for deep links.
  Pagination and `total` are at the article level; article order is "best-hit"
  order. Times are ISO8601 UTC.
- Search contract (unchanged): lowercase enum serialization, dispatch agreement
  via `is_search_request`, `search_page_size=8`, `max_search_pages=1024`,
  `<mark>` highlight convention.

## Listing, ordering, and pagination

- **No ordering in listing queries (agdb).** `versions_of` drops all ordering
  (DB `order_by` and Rust-side `sort`). agdb's `elements()` scan is
  storage-slot ordered; removing `order_by` enables the `LimitOffsetHandler`
  short-circuit (O(offset+limit) instead of a full scan). Order is therefore
  insertion/storage order, not time or id order — accepted.
- **Dead list interfaces are deleted, not kept.** `read_articles` (the plain
  list branch) and `read_users` were frontend-unused; they are removed outright.
  `/user/read` is deleted; `/article/read` now serves only the full-text search
  path (`search_articles`) — a bare `/article/read` no longer returns a list.
  `read_article` (single read by id) and `versions_of` remain, as the frontend
  uses them.
- **User-facing sort is removed.** The full-text search `sort` controls (time /
  title / author, direction, URL serialization) are deleted as a product feature.
- **Total page count is kept.** List responses still return `total` /
  `total_pages` for jump-to-page and `"/ {total_pages}"` display. Counting the
  total requires one full scan of the type (O(A)/O(U)) and is an accepted,
  irreducible cost under the current storage model. Search-page `total` is
  unaffected (it comes from SeekStorm, no agdb scan).

## Performance remediation (open plan)

- **`enrich_articles` must be localized (pending).** The shared enrich helper
  (`read_article`) builds three all-library HashMaps (author edges, tag edges)
  per call. A companion to the ordering removal. Fix: query the author/tag edges
  with `where_.ids(article_ids)` and batch `read_rows`, giving O(page·degree)
  instead of O(A+edges). No behavior change.
- **`find_document_ids_by_article` is intentionally left alone.** It is on the
  non-hot rebuild path and already uses an index facet; any real gain lives in
  the SeekStorm layer, out of scope.

## Conventions

- **URL updates from reactive state use `replace: true`** (draft persist, search
  sync, pagination). A pagination click must replace, not push, a history entry.
- **Dev database is reset and reseeded at startup**; no migration scripts while
  the system is not live. Schema bumps force a full search-index rebuild
  (`sync_all`).