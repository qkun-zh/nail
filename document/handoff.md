# Handoff

## Current state

Search page is live and healthy: backend `:3000`, proxy `:8080`, SPA all `200`.
Front tests 76/76, back tests 310/310, common 110/110; clippy warnings are all
pre-existing in `comment.rs`/`seed.rs` (back) and component `children`/
`error`/`loading`/`roots`/`target` redefinitions (front).

## What was done

- Rebuilt `code/front/src/page/public/article/search.rs` to match the
  `document/search-preview.html` template (STYLE, label-chip, pagination).
  URL carries query params + article page only; hit versions and comments
  paginate client-side (`LocalPagedList`, 8 per page) per the adjudicated
  plan (ADR-0006).
- `document.rs::read_string_field` returns `to_string()` for non-string JSON
  values (tags stored as a JSON array). Fixes the "tag always displayed" bug.
- `repository/search.rs::read()` skips comment documents on empty queries
  (`enable_empty_query`), so browsing mode shows article cards only. Fixes the
  "empty query shows version/comment cards" bug.
- Front `SearchVersions` now renders `SearchComments` only when the version's
  comment list is non-empty. Fixes the "empty comment card" bug (search "58").
- Search result links now use `version_id` in the URL path (was
  `version_number`), matching the router's `/version/:version_id` route. Also
  fixed the comment author rendering to use `inner_html` so `<mark>` highlights
  display instead of leaking as literal markup.
- Renaming a user now also re-syncs the search documents for articles the user
  commented on (`article_ids_of_user` walks user -> comment -> version ->
  article), not just the articles they authored. Fixes "renamed user's
  comments still searchable under old name". Green test:
  `sync_user_refreshes_the_author_name_of_their_comments` in
  `test/unit/back/repository/search.rs`.
- Removed the `single_char_query_marks_the_matching_version_number` red test:
  single-character queries deliberately skip `<mark>` insertion inside
  SeekStorm (`no_score_no_highlight`, `highlighter.rs`), so no highlight and no
  `version_number_hit` on single-char queries is intended SeekStorm behaviour,
  kept as-is by user decision.
- Both fixed bugs have green tests in `test/unit/back/logic/search.rs`
  (`empty_query_does_not_surface_version_or_comment_cards`,
  `keyword_that_misses_tags_does_not_report_a_tag_hit`).
- Search page is now search-type, not browse: an empty query short-circuits on
  the frontend (`do_search` in `search.rs` clears results and shows "enter a
  query to search") instead of returning all articles. Backend empty-query
  browse capability is untouched (still tested). Backend test
  `empty_query_does_not_surface_version_or_comment_cards` keeps passing.
- Removed reactive_graph warnings in `search.rs`: the one-shot reads at
  component build (`limits.get()`) and inside `do_search` (`per_page`,
  `q_filter`, `ranges`, `sort_order`, `from_time`, `to_time`) now use
  `get_untracked()` instead of `.get()`, matching their intended non-reactive
  snapshots.

## Known behaviour (user-accepted, documented here)

- Single-character queries (e.g. "1") return article cards but no `<mark>`
  highlighting and no version/comment cards: SeekStorm's
  `no_score_no_highlight` optimisation. Not a bug.
- Hyphenated author names ("sample-author-00") tokenise as one token, so
  searching "author" finds nothing while "auth" matches via the `auth` tag and
  substring-highlights the author name. SeekStorm tokenizer behaviour,
  kept as-is.

## Next steps

- User verification: hard-refresh the search page. Empty query now shows
  "enter a query to search" (search-type) instead of all articles; a non-empty
  query returns matching results.
- Restart procedure lives in `document/run.md` (three `200`s).