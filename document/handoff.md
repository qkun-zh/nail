# Handoff

## Current state

All three crates (`common`, `back`, `front`) pass `cargo clippy` with
**zero warnings** under the strictest gate: `clippy::pedantic` denied at crate
level, `clippy::too_many_lines` exempted, plus `cargo fmt --check` clean.
Test code is deliberately **outside** the clippy gate: run plain `cargo clippy`
(no `--all-targets`), so `#[cfg(test)]` modules are not compiled and not
linted (an in-code `#![allow]` cannot override the Cargo.toml `deny`).
Search page is live and healthy: backend `:3000`, proxy `:8080`, SPA all `200`.

## What was done

- Search slice (committed `d78a5b5`): empty query and empty ranges now return
  empty results instead of falling back to all fields; `field_hit` in
  `repository/search/document.rs` switched from `<mark>` presence to a
  case-insensitive substring scan of the folded raw value against SeekStorm's
  official `query_terms` hook (`ResultObject.query_terms`); the frontend always
  sends the `ranges` parameter (all 7 fields, a subset, or empty string), and
  the previously-known single-character no-hit-card behaviour is fixed — "9"
  now reports Summary hit cards. All 312 back tests green, clippy clean.
  `document/run.md` rewritten into stepwise start/health-check instructions.
- Union search slice (this working tree): default semantics switched from
  Intersection (AND) to Union (OR) in the three `QueryType` call sites of
  `repository/search.rs` (~202 sync_all, ~294 read, ~365
  find_document_ids_by_article). Plain space now ORs; `+word` marks a term
  required (AND), `-word` excludes, `"a b"` is a phrase. Frontend search box
  placeholder updated with that syntax. New test
  `space_separated_keywords_match_any_field_or` asserts result sets (total +
  `<mark>`-stripped titles) for OR / `+` AND / single-`+` required / `-`
  exclude. Live-verified on the real index: "scheduler parser" = 58 (OR),
  "scheduler +parser" = 1 (AND), "async +queue" = 0.
- **SeekStorm `+` query_terms caveat (investigated to the source, important
  for future highlight work)**: `ResultObject.query_terms` is built per shard
  inside `search_lexical_shard` (seekstorm `src/search.rs`), and an
  Intersection-flagged term (`+word`) whose posting list is absent from a
  shard hits `break 'fallback` (line ~3293), truncating the term loop so later
  terms never reach the `query_terms` push. The top-level result then takes
  the first shard's non-empty `query_terms` (line ~1885). With the default
  shard count (CPU count; our test index runs 4 shards), `+`-prefixed terms
  can therefore be missing from `query_terms` depending on shard layout —
  empirically `+alpha +beta` yielded `["alpha"]` with one fixture layout and
  `["alpha","beta"]` after adding one more article. Result *sets* are stable
  (semantics correct); only highlight/hit-card completeness for `+` terms is
  affected. Tokenizer itself is correct (`+alpha` → `op=Intersection`,
  `["+alpha", "+beta"]` split verified by source-level debug). Tests must
  therefore assert result sets, not `<mark>` spans, for `+`-containing
  queries.
- Set the repo clippy standard to the strictest level: `[lints.clippy]` with
  `pedantic = { level = "deny", priority = -1 }` and `too_many_lines = "allow"`
  in all three `Cargo.toml` files. Fixed every pedantic warning across the
  three crates (common ~50, back ~76, front ~25); the only exempted class is
  overlong functions (`too_many_lines`).
- Panic-free conventions used throughout: integer casts use
  `usize::try_from(x).unwrap_or(usize::MAX)` / `i64::try_from(x).unwrap_or(0)`
  etc.; `{path:?}` debug formatting replaced with `.display()`; redundant
  `Result` wrappers removed from `graph::open`, `delete_session`,
  `insert_session`, `OffsetTime::new`, and the search/comment read outcomes;
  duplicated enum field names de-hashed in `EmailUpdateTokenEntry`; async
  functions that never await were made sync and their call sites updated.
- Front: `apply_fallbacks` now takes `&RuntimeLimits`; JS `f64 -> u64`
  conversions centralised in the new `infrastructure/js.rs`
  (`js_number_to_u64`, with `#[allow]` for the two inherent float-cast lints
  since std has no `TryFrom<f64>` for integers); `map().unwrap_or()` ->
  `map_or()`; `if !x {} else {}` swapped to `if x {} else {}`; a couple of
  casts (`toast_duration_ms as u32`, `max_tags_per_article as usize`) use
  `try_from`.
- Test crates are included in the clippy gate (`--all-targets`); tests are not
  run as part of this pass (user: "test不必管").
- Per user request ("test不要clippy介入"), test code is exempt from the clippy
  gate: the gate command is plain `cargo clippy` (no `--all-targets`), so
  `#[cfg(test)]` modules are neither compiled nor linted. The earlier clippy
  fixes inside test files were left in place (harmless).

- Notification refactor (committed `5757b73`): `page/notify.rs` rebuilt as a
  fixed top-center toast overlay. Four kinds map to four accent colours
  (Info=blue, Success=green, Warning=yellow/amber, Error=red) via
  `toast--{kind}` classes; every kind auto-dismisses after a uniform 4s
  (`TOAST_DURATION_MS`, one gloo `Timeout` per toast). History storage,
  the 1s countdown ticker, the dismiss/history buttons and `remaining_seconds`/
  `toast_duration_ms`/`capped_insert` are gone. CSS lives as a namespaced
  `toast-*` `<style>` block in `code/front/index.html` (the toast overlay is
  global infrastructure, not a page, so the README §8 no-CSS-on-pages rule
  does not apply); the container is `position: fixed` with `pointer-events:
  none` and high `z-index`, so it neither reflows nor blocks page interaction.
  Notify call sites unchanged. Tests rewritten to cover the 4s constant and
  the kind→class mapping; all 74 front tests green, clippy clean, `trunk
  build` succeeds.

## Known behaviour (user-accepted, documented here)

- Single-character queries (e.g. "9") are handled via `query_terms`
  substring matching, so hit cards are now reported even though SeekStorm's
  `no_score_no_highlight` optimisation skips `<mark>` insertion for them.
- Hyphenated author names ("sample-author-00") tokenise as one token, so
  searching "author" finds nothing while "auth" matches via the `auth` tag and
  substring-highlights the author name. SeekStorm tokenizer behaviour,
  kept as-is.

## Next steps

- User verification: `cargo clippy` (zero warnings) and
  `cargo fmt --all -- --check` inside `code/{common,back,front}`.
- Restart procedure lives in `document/run.md` (stepwise, health checks per
  component).