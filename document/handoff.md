# handoff

> Rule: handoff stays lean. Once an independent task is fully done (committed
> and verified), remove it from this file — don't let completed items pile up.
> Keep only: current state, tasks not yet finished, and unresolved decisions.
> If the file stops being a compact handover, prune it.
> Rule: any code change that introduces a new scenario (new behavior, branch,
> input domain, or edge case) MUST add matching formal tests to the real suite
> — more than 3 tests per new scenario. A scenario without >3 formal tests is
> incomplete.

## State

- Backend (axum) + frontend (Leptos CSR) + proxy (pingap) knowledge base; agdb
  graph, SeekStorm search, email-challenge + PoW auth, Cedar authz.
- Uncommitted (other agent): `document/workflow.md`, `AGENTS.md`, `README.md` —
  the double-evidence workflow refactor (see Done).
- **Soft-delete refcount + restore API (committed bac4e65, c40608b, 6c33fac,
  97fd467, 6883a5b)**: done — `KEY_SOFT_DELETED` is a u64 count, soft-delete
  cascades `+1` over the subtree, restore `-1` (key deleted at 0; invariant key
  present ⟺ hidden, so read filters unchanged). Read path is a single O(1)
  self-flag check (ancestor-chain walk deleted). Restore is admin-only via 3 new
  actions on independent `POST .../restore` routes; repeated soft-delete rejected
  (`bad_request`). 424 back tests pass, clippy clean. See Done for measurements.

## Done

- **Soft-delete refcount + admin-only restore (committed bac4e65, c40608b,
  6c33fac, 97fd467, 6883a5b)**: `KEY_SOFT_DELETED` refactored from a bare flag to a
  subtree count (`adjust_soft_delete_count` cascades ±1 over article/version/comment
  subtrees; key removed at 0; missing key reads as 0). Read path (`comment.rs`,
  `version.rs`, `search/document.rs`) checks the target node's own flag in O(1)
  instead of walking the ancestor chain — 13 call sites converged, the two
  `content_path_soft_deleted_*` functions and 5 helpers deleted (−299 lines).
  Repeated soft-delete returns `bad_request("already soft-deleted")` at the logic
  layer (storage stays pure counting). 3 restore actions added to `ALL_PERMISSIONS`
  + `schema.cedar`; seed grants them only to admin, and the owner bypass does not
  list them, so members (even owners) get 403. Routes
  `POST /{article|version|comment}/{id}/restore` decrement the subtree and re-sync
  the search index; restoring a visible node returns `bad_request("not soft-deleted")`.
  3 repo tests flipped to cascade assertions; 17 new logic/http tests. 424 back
  tests pass, clippy clean. Release perf probe (202-node subtree): soft-delete
  1.29 ms, restore 0.91 ms, per-item soft-delete check ~1.1 µs (was up to 11
  queries/item on the old chain walk).
- **Workflow refactor**: added double evidence (source + probe) for every
  unknown and a Phase 5.5 adoption gate — no code until evidence is presented
  and the user adopts the plan. Synced wording in `AGENTS.md` and `README.md`.
  `document/decisions.md` abolished; removed all references.
- **Dead-interface deletion**: removed frontend-unused `read_articles`
  (plain-list) and `read_users` across `repository`, `logic`, `interface`,
  `router`, and `common/response`, plus their tests. 304 tests pass.
- **P2/P3 perf refactors (committed 20fdeb4, cf701c4)**: localized `enrich_articles`
  (`repository/article.rs`, targeted `.to()/.from()` + batch `read_rows`, O(E)→O(1));
  and batched `enrich_comment_headers` (`repository/search.rs`, per-comment round trips
  → O(#distinct ids) resolves + constant batch reads). Probes verified identical output
  and refuted the old `where_.ids` plan. 306 tests pass. Tracked in
  `document/performance-refactor.md`.
- **P5 list pagination (committed c2f62ec)**: `versions_of` and both comment paginators
  now use a single agdb `.offset().limit()` query (default storage order), dropping the
  newest-first sort and the `total` count; `has_next` from a `limit+1` peek. DTOs drop
  `total` (`VersionListPage`, `CommentListPage`) and `VersionListItem.created_at`
  (unused). Frontend uses a new `PrevNext` control instead of numbered pagination.
  308 tests pass; frontend trunk build clean.
- **Search ORDER BY removed (committed af09b00)**: dropped the client-selectable
  time/title/author sort on `/search` (common `SearchSort*`, request param,
  `parse_sort`, `ResultSort` → SeekStorm, frontend sort chips). Results always come in
  SeekStorm default order (BM25 relevance for keyword search); sort UI removed.
- **Soft delete (committed 752cdae, 3a5d274, 8ef805b, abd2f26, 3ac2257)**: W0 mechanism +
  plumbing (`DeleteMode::Soft`, `KEY_SOFT_DELETED`, `soft_delete_*`, `*_DELETE_SOFT`
  permissions, cedar policy 1/3, frontend soft UI); W1 read filtering (articles,
  versions incl. latest fallback, comments — replies stay visible); W2 search index
  exclusion (version doc dropped but its comments indexed; article flag never hides
  children docs); W5 tests: logic (article/version/comment soft delete, member-owner
  version bypass, held title/hash), http envelopes, restore mechanism
  (`clear_soft_deleted_flag` revives a node). 3ac2257 scoped member
  transfer/soft-delete to self-owned content (removed the global member grants from
  `seed.rs`; owner bypass is the only member path). Gates green at each commit:
  back 340, common 109, frontend trunk build clean.

## Next

- Commit the uncommitted slices (one commit each, clean tree).
- Perf: P2, P3, P5, search-ORDER-BY closed. P1 rejected (highlight behavior);
  P6 non-problem (O(R)); P4 accepted (inherent). Open: total/cursor on list endpoints
  + search total.