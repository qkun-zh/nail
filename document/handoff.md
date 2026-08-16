# handoff

## Current state

- Backend (axum) + frontend (Leptos CSR) + proxy (pingap) knowledge base; data in
  agdb graph, search in SeekStorm, auth via email challenge + PoW, authorization
  via Cedar.
- Working tree has uncommitted changes (this slice) on `document/workflow.md`,
  `AGENTS.md`, and `README.md` (§12) — the double-evidence workflow refactor
  below. The earlier dead-interface slice (below) is still uncommitted as noted.

## What was done

- `document/workflow.md`: refactored the execution loop to require **double
  evidence** (source evidence from pinned library source / repo modules, plus
  probe evidence from a disposable test) for every unknown before any code is
  written, and added **Phase 5.5 `evidence_gate`**: the plan is presented to the
  user with its evidence and implementation waits for explicit adoption. Updated
  the top-level loop diagram, `plan` (phase 4 names unknowns), the when-mandatory
  table, the loop-back rules, and the invariants. `AGENTS.md` and `README.md` §12
  synced to the same wording (source + probe evidence, adoption before
  implementation).
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
  dropped; `enrich_articles` localization now used only by `read_article`).
- **P2 perf refactor (committed 20fdeb4)**: localized `enrich_articles` in
  `repository/article.rs` — replaced full-graph owner/tag edge scans with targeted
  `.to()/.from()` queries per article plus batch `read_rows`. `read_article` drops
  from O(total edges) to O(1). Probe (`test/unit/back/repository/probe.rs`,
  `probe_targeted_queries_localize_by_endpoint`) verified identical edge ids; it
  also refuted the old `where_.ids` plan (agdb `ids` matches element id, not
  endpoints). 305 tests pass. Plan tracked in `document/performance-refactor.md`.

## What comes next

- Commit this slice on a clean tree (workflow: one commit per slice). Not yet
  committed.
- Soft delete (mode Soft, delete-flag scheme 1) remains unimplemented/decided.
- Perf refactor: P2 (`enrich_articles` localization) done. P3
  (`enrich_comment_headers` batching) approved, awaiting source+probe evidence before
  implementation. P6 (recycler HashSet) **not approved** by user. P1/P5/total-cursor open.
  P2's problem/solution removed from `performance-refactor.md` tracking (code kept).