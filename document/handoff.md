# handoff

## Current state

- Backend (axum) + frontend (Leptos CSR) + proxy (pingap) knowledge base; data in
  agdb graph, search in SeekStorm, auth via email challenge + PoW, authorization
  via Cedar.
- Working tree clean. Latest commit baseline green (`cargo fmt`, `cargo clippy`,
  `cargo test` pass in `code/{common,back,front}`).

## What was done

- Renamed agdb edge constants from prepositional `X_to_Y` to subject-verb-object
  `X_verb_Y` form (schema.rs + 9 referencing files, incl. tests):
  - `user_to_article` -> `user_author_article`
  - `article_to_version` -> `article_hold_version`
  - `user_to_comment` -> `user_author_comment`
  - `comment_to_version` -> `comment_attach_version`
  - `comment_to_comment` -> `comment_reply_comment`
  - `article_to_tag` -> `article_apply_tag`
  - Unchanged (already SVO): `user_hold_role`, `role_grant_permission`,
    `role_apply_tag`.
  - Pure mechanical rename; data/agdb resets/reseeds at startup so no migration.

## What comes next

- None pending from the rename slice.