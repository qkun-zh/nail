# Handoff

## Current state

All three crates (`common`, `back`, `front`) pass `cargo clippy --all-targets`
with **zero warnings** under the strictest gate: `clippy::pedantic` denied at
crate level, `clippy::too_many_lines` exempted, plus `cargo fmt --check` clean.
Search page is live and healthy: backend `:3000`, proxy `:8080`, SPA all `200`.

## What was done

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

## Known behaviour (user-accepted, documented here)

- Single-character queries (e.g. "1") return article cards but no `<mark>`
  highlighting and no version/comment cards: SeekStorm's
  `no_score_no_highlight` optimisation. Not a bug.
- Hyphenated author names ("sample-author-00") tokenise as one token, so
  searching "author" finds nothing while "auth" matches via the `auth` tag and
  substring-highlights the author name. SeekStorm tokenizer behaviour,
  kept as-is.

## Next steps

- User verification: `cargo clippy --all-targets` (zero warnings) and
  `cargo fmt --all -- --check` inside `code/{common,back,front}`.
- Restart procedure lives in `document/run.md` (three `200`s).