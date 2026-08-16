# handoff

## Current state

- Backend (axum) + frontend (Leptos CSR) + proxy (pingap) knowledge base; data in
  agdb graph, search in SeekStorm, auth via email challenge + PoW, authorization
  via Cedar.
- Working tree has uncommitted changes (this slice) on `document/decisions.md`
  plus the dead-interface deletion below. Baseline green confirmed after the
  deletion: `cargo fmt`, `cargo clippy --all-targets` (zero warnings), and
  `cargo test` (304 passed) in `code/back`.

## What was done

- Deleted the frontend-unused list interfaces `read_articles` (plain-list
  branch) and `read_users`:
  - `repository/article.rs`: removed `read_articles` (agdb `order_by` list scan).
  - `repository/user.rs`: removed `read_users` and its `UserListItem`.
  - `logic/article.rs`: removed `read_articles`, `ArticleReadPage`,
    `is_search_request`, and the List branch; `search_articles` stays.
  - `logic/user.rs`: removed `read_users`.
  - `interface/article.rs`: `read_articles` handler -> `search_articles` (direct
    `logic::search::search_articles`, returns `SearchPage`).
  - `interface/user.rs`: removed `read_users` handler + `UsersReadParams`.
  - `router.rs`: deleted `/user/read`; `/article/read` now routes to
    `search_articles` (bare `/article/read` returns no list).
  - `common/response/{article,user}.rs`: removed `ArticleListItem`,
    `ArticleListPage`, `UserListItem`, `UserListPage`.
  - `repository/schema.rs`: removed unused `KEY_ID` const.
  - Removed the matching unit tests (repo/user.rs, logic/user.rs,
    repository/article.rs, http/article.rs plain-list + clamp tests,
    http/user.rs list tests). 304 tests still pass.
- `document/decisions.md`: recorded the dead-interface deletion and updated the
  ordering/perf entries (the `read_users` short-circuit item is obsolete and was
  dropped; `enrich_articles` localization still pending, now used only by
  `read_article`).

## What comes next

- Commit this slice on a clean tree (workflow: one commit per slice). Not yet
  committed.
- Soft delete (mode Soft, delete-flag scheme 1) remains unimplemented/decided.
- `enrich_articles` localization (perf open plan) still pending.