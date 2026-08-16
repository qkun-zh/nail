# ADR-0006: Search — version-indexed article tree (design)

## Status

Accepted and implemented (2026-08-16): the version-indexed index, the
two-document-shape schema, and the hierarchical response contract described
below are live in the current code.

## Context

The design replaced an earlier search that indexed **one document per article**,
aggregating a whole article — including *all* versions' comments — into a single
SeekStorm document. Concrete failures of that shape:

1. **Comment field bloat.** Every version's comments are concatenated into the
   article document. Index build, sync, and decompression are O(all comments)
   even when a single comment hits.
2. **Multi-value fields are not hit-addressable.** `tag`/`comment` are stored
   as `FieldType::Json` arrays. SeekStorm's highlighter
   (`highlighter.rs:237-252`) recursively flattens a Json field's values into
   one string and scans it, so a hit on one element returns the whole array
   (non-hit elements included); `snippet.contains("<mark>")` is a partial hack.
3. **The response contract is flat, the UI is a tree.** A flat
   `SearchArticleItem { id, title, author, time, hits }` / `SearchHit { field,
   label, snippet }` cannot express "article → version → comment" hierarchy,
   cannot deep-link versions/comments (no `version_id`/`comment_id`), and does
   not carry per-entity metadata (version number, per-version time, comment
   author/time).

The frontend page and its search controls are fixed (frontend `search.rs`:
query box, range checkboxes, from/to ISO8601 time bounds, sort
time/title/author, project `Pagination` component).

## Hit propagation semantics (the display contract)

A node appears in the result tree **iff** it or any descendant hits:

- An **article** appears iff any of its fields hits (`summary`, `tag`,
  `author_name`, `title`) **or** any of its versions' fields hits
  (`version_number`, `note`) **or** any comment under those versions hits.
- A **version** card appears iff that version has a field hit (`version_number`,
  `note`) **or** any of its comments hits.
- A **comment** card appears iff that comment hits.

Display rules:

| Node | Always shown (default) | Shown only on hit (with field label) |
| --- | --- | --- |
| article | `title`, `author_name`, `time` | `summary`, `tag` |
| version | `version_number`, `time` | `note`, `comment` (→ comment card list) |
| comment | — | author + time + content (card) |

- "Always shown" fields render unconditionally; when they hit, their match
  terms are `<mark>`-highlighted. A hit on `version_number` highlights the
  version number chip; a hit on `author_name` highlights the author name.
- The version card is only rendered when the version itself has a hit (see
  propagation); the version number is then a default-shown, possibly-marked
  value — it never renders a version card by itself.
- `comment` is an entity (author, time, content), rendered as a full card, not
  a single-line snippet. Comments are flat under a version: no reply nesting,
  no "replies to X" affordance; each is independent.
- The `comment` field label is outer-level (same indent as `note`); its value
  is a comment-card list with its own internal pagination.

## Decision

### Index: one SeekStorm index, one document per version + one per comment

SeekStorm schema fields are **optional per document** ("Even if
`index_lexical` is true in the schema, the field in the actual document is
optional", `index.rs` SchemaField docs). Two document shapes share the index,
distinguished by a `doc_type` facet field. A `StringSet16` `doc_type` allows
filtering either shape with a `FacetFilter`.

**Version document** (a version = searchable unit):

| field | FieldType | store | index | facet | boost | notes |
| --- | --- | --- | --- | --- | --- | --- |
| `doc_type` | StringSet16 | false | false | true | — | `["version"]` |
| `version_id` | String32 | true | false | true | — | document identity, deep-link |
| `article_id` | String32 | true | false | true | — | collapse key |
| `version_number` | Text | true | true | false | 2.0 | searchable range |
| `title` | Text | true | true | false | 3.0 | denormalized from article |
| `summary` | Text | true | true | false | 1.0 | denormalized |
| `author_name` | Text | true | true | false | 2.0 | denormalized |
| `note` | Text | true | true | false | 1.0 | this version's note |
| `tags` | StringSet16 | true | true | false | 1.0 | denormalized tag names |
| `ts` | Timestamp | true | false | true | — | `uuidv7_timestamp_secs(version_id)` |

**Comment document** (one per comment):

| field | FieldType | store | index | facet | boost | notes |
| --- | --- | --- | --- | --- | --- | --- |
| `doc_type` | StringSet16 | false | false | true | — | `["comment"]` |
| `comment_id` | String32 | true | false | true | — | deep-link |
| `version_id` | String32 | true | false | true | — | parent version |
| `article_id` | String32 | true | false | true | — | collapse key |
| `author_name` | Text | true | true | false | 1.0 | comment author |
| `content` | Text | true | true | false | 1.0 | comment body |
| `ts` | Timestamp | true | false | true | — | `uuidv7_timestamp_secs(comment_id)` |

- **Why version + comment documents, not one version document with embedded
  comments:** highlighting cost is per-field per-document. Embedding comments
  in the version doc forces the highlighter to decompress + flatten the whole
  comment array even when one comment hits (`highlighter.rs:237-252`). A
  separate comment doc keeps the highlighter's input to a single short field.
- `ts` is **not** a persisted entity field; it is derived from the id's
  UUIDv7 timestamp (all ids are `Uuid::now_v7()`). Article `time` (article
  header) = `uuidv7_timestamp_secs(article_id)`; version `time` =
  `uuidv7_timestamp_secs(version_id)`; comment `time` =
  `uuidv7_timestamp_secs(comment_id)`.
- `content_hash` is not indexed. The PDF is not searchable content; it is a
  payload discovered via peripheral fields, download keeps the existing token
  flow.
- `version_number` uses `Text` (not `String16`/facet): it is user-readable,
  tokenized (e.g. `1.4.0`), and does not need string faceting.

### Searchable ranges

Every user-readable text field is searchable. Range → field name:

| Range key | Field (document shape) | Layer |
| --- | --- | --- |
| `title` | `title` (version) | article |
| `summary` | `summary` (version) | article |
| `author_name` | `author_name` (version) | article |
| `tag` | `tags` (version) | article |
| `version_number` | `version_number` (version) | version |
| `note` | `note` (version) | version |
| `comment` | `content` (comment) | version |

`SearchRange` gains a `VersionNumber` variant. The field is named
`author_name`, not `author`.

The other search contracts are **not** stale and stay as-is: param shape
(`ArticleSearchParams{q,ranges,sort,from,to,limit,page}`, all optional),
lowercase enum serialization (`title/summary/author/comment/note/tag`,
`time/title/author`, `asc/desc`), dispatch agreement (`is_search_request`),
config (`search_page_size=8`, `max_search_pages=1024`), `<mark>` highlight
convention, ISO8601 times.

### Query

One SeekStorm query per request, searching both document shapes at once:

- `query_string` from the `q` box; `enable_empty_query=true` when `q` is empty
  (list mode).
- `field_filter` = enabled ranges' field names (e.g. `["title","summary",…]`).
  A version doc only has version-shape fields; a comment doc only has
  `content` — `field_filter` naturally restricts which shape a term hits.
- `facet_filter` = `Timestamp { field: ts, from..to }` when time bounds set.
- `result_sort` = the frontend sort mapping (`ts`/`title`/`author_name`).
  `title`/`author_name` facets sort on version docs; comment docs without those
  fields are ignored by facet sorting (SeekStorm sorts only docs that carry the
  facet field).
- `result_type = TopkCount` (total + top-k), `realtime = true` (matches current
  behavior).

### Collapse + pagination in the logic layer

SeekStorm has **no collapse/dedup** (verified: no group-by in the API). The
logic layer folds hits by `article_id` after the single query:

1. Group version hits and comment hits by `article_id` → article tree.
2. Attach comment hits under their `version_id`'s version node.
3. A version node exists in the tree iff it has a version hit or ≥1 comment
   hit (propagation). An article node exists iff it has any hit or ≥1 version
   node (propagation).
4. **Article order** = the order SeekStorm returned the article's first
   (highest-scoring) hit, i.e. articles are ranked by their best hit. Since
   page size is small and the result is top-k by score, pagination slices the
   collapsed article list at the article level. (SeekStorm top-k limits the
   number of docs the logic layer must fold — folding never processes the whole
   index.)

Page semantics: `page`/`limit` apply to **articles** (not hits). The frontend
`Pagination` component drives this. `total` = collapsed article count
(computed from returned hits; SeekStorm's `result_count_total` counts docs,
so the logic layer reports the article-level count, which is what the UI shows).

### Highlighting (cost-controlled)

- One `highlighter()` per request over the returned top-k hits only — never
  over the whole result set.
- A `Highlight` is built **only for the enabled ranges' fields** (`effective`
  set), mirroring today's `repository/search.rs:243-254`.
- `fragment_size=4096` guards against a single over-long field; it does **not**
  protect against reading big embedded arrays — that protection comes from the
  comment-is-a-doc split above.
- Cost is proportional to `top-k × enabled ranges`, bounded by the returned
  page; deep pages or dense hits scale linearly but stay O(page).

### Response contract

Rework `common/src/response/search.rs` from flat to hierarchical:

```
SearchPage { article_list, total, page, total_pages, has_next, has_prev, truncated }
SearchArticleItem {
  article_id, title, author_name, time,        // always shown, may contain <mark>
  article_hits: Vec<SearchHit>,                // summary, tag
  versions: Vec<SearchVersionItem>,
}
SearchVersionItem {
  version_id, version_number, time,            // always shown, may contain <mark>
  version_hits: Vec<SearchHit>,                // note; version_number highlight goes on version_number
  comments: Vec<SearchCommentItem>,            // comment cards (paginated client-side per version)
}
SearchCommentItem { comment_id, author_name, time, content }
SearchHit { field, label, snippet }            // snippet already <mark>-highlighted
```

- Comment pagination lives inside a version's comment list, reusing
  `COMMENTS_PER_PAGE` (8) and the project `Pagination`/`LevelPagination`
  pattern; page size default matches the frontend limits.
- Deep links: article → `/public/article/{article_id}`; version →
  `/public/article/{article_id}/version/{version_id}`; comment →
  `/public/article/{article_id}/version/{version_id}/comment/{comment_path}`.
  Because titles/version numbers/author names may contain `<mark>`, they are
  **not** anchors; the field-name chips and the comment author row are.

### Times

All rendered times use ISO8601 UTC (`format_rfc3339_utc`, `…Z`), matching
`format_search_time` in `logic/search.rs`.

## Consequences

- Sync/sync_all rebuilds both doc shapes: one version doc per version, one
  comment doc per comment. A version update rewrites its own doc; a comment
  update rewrites one comment doc. Index size drops from O(all comments per
  article) to O(comments) with no article-level duplication.
- The response contract is a **breaking change**: flat hits became a tree.
  `logic/search.rs` (collapse, ranges, `VersionNumber`, article-count
  pagination), `repository/search.rs` + `document.rs` (two doc shapes), and
  frontend `page/public/article/search.rs` (tree render, deep links, per-version
  comment pagination) were reworked accordingly.
- Requires a full index rebuild on deploy (`sync_all`).
- Caveat (accepted): article ordering is "best hit order", not a separate
  article-level relevance key; with top-k limiting this is bounded and
  predictable.

## Visual contract

The result layout is a three-level card tree: article cards with field-name
links, version cards, and comment cards (author/time/content) with internal
comment pagination; ISO8601 UTC times; version number without a `v` prefix.
