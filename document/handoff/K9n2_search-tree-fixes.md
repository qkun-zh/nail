# Handoff

## Task I: Search result-tree fixes (#1 duplicate hits, #2 author link)

**Owner**: K9n2
**Exec doc**: (deleted — task complete)
**Status**: Complete — both slices done, tests promoted, final gate green

### Stages

A. ✅ #1 — Dedup article-level hits across versions (Slice 1)
B. ✅ #2 — Keep article author id on comment-only hits (Slice 2)
C. ✅ Promote regression tests + final gate

### Evidence (source + probe)

- #1: `logic/search.rs:130` appended each version's `article_hits` to the shared
  article → duplicate `summary`/`tag` cards. Probe red (`["summary","summary"]`).
- #2: `logic/search.rs` comment branch set `author_id: String::new()` because
  `SearchCommentOutcome` carried no article author id → broken `/user/` link.
  Probe red (`author_id=""`).

### Decisions

- Dedup `article_hits` by whole `SearchHit` in `assemble_tree`.
- Add `article_author_id` to `SearchCommentOutcome`, filled in
  `enrich_comment_headers` from the existing author edge lookup.
- #3 (pagination window) rejected by user — keep as-is.
- #4 (version-number highlight ordering) deferred by user — source-only, not
  deterministically probe-able.
- #5 (id/role hits unannotated) accepted as-is by user.

### Code changes

- `code/back/src/logic/search.rs` — dedup article hits; use `article_author_id`.
- `code/back/src/repository/search.rs` — `SearchCommentOutcome.article_author_id`.
- `code/back/src/repository/search/db.rs` — fill `article_author_id`.
- `code/back/src/repository/search/document.rs` — init the new field.
- `test/unit/back/logic/search_verify.rs` — two new regression tests.
- `test/unit/back/harness.rs` — transient probe wiring added then removed.

### Final gate

- ✅ `cargo test` (back) — 543/543 pass
- ✅ `cargo clippy --bin nail_back --all-targets -- -D warnings` — zero warnings
- ✅ `cargo fmt --check` — clean for all files touched by this task
- ⚠ Pre-existing fmt drift (not this task) remains in
  `test/unit/back/infrastructure/probe_002_orthogonal_action_resource.rs` and
  `test/unit/back/logic/probe_review_findings.rs` (other agents' files, untouched).