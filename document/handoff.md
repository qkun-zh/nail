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
- **Authz refactor (in progress, plan `document/authz-refactor.md`)**: A1 done
  (`8698ecc`), A2 done (`208e94c`), A3 done (`9a92e1e`), A4 done (`0d3e7de`),
  A5 done (`2b1f02c`), A6 done (docs, `document/authz.md`), B0 done (baseline
  probe `probe_001`), B1 done (`295ff6e`). Next: B2 (`read_user` self-view to
  Cedar).
- **Soft-delete refcount + restore API (committed bac4e65, c40608b, 6c33fac,
  97fd467, 6883a5b)**: done — `KEY_SOFT_DELETED` is a u64 count, soft-delete
  cascades `+1` over the subtree, restore `-1` (key deleted at 0; invariant key
  present ⟺ hidden, so read filters unchanged). Read path is a single O(1)
  self-flag check (ancestor-chain walk deleted). Restore is admin-only via 3 new
  actions on independent `POST .../restore` routes; repeated soft-delete rejected
  (`bad_request`). 424 back tests pass, clippy clean. See Done for measurements.

## Done

- **Authz B1 — read enforcement through Cedar (committed 295ff6e)**: threaded
  `actor_id` through the read logic and interface handlers (`read_article`,
  `search_articles`, `read_version`/`read_versions`,
  `read_comments`/`read_comment`/`read_comment_children`). Single-resource reads
  gate with `authorize_or` against the resource (not-found message wins);
  collection reads gate once with `authorize` against the coarse
  `Virtual::"read"` desk, placed before validation (fail-closed). Seeded the D5
  member read grants (`Article::Read`/`Version::Read`/`Comment::Read`), promoted
  `PERMISSION_ARTICLE_READ`/`PERMISSION_COMMENT_READ` to production, deleted
  read-open policy 2 (policy numbering stable). Red: 8 interface tests (403
  expected, 200 observed pre-change). Green: 446 back tests; new logic-level
  denial tests for all seven read functions; the cedar read-open test rewritten
  to `read_requires_a_role_grant`; `logic/authorize.rs` read-open test renamed
  `member_can_read_articles_and_versions_via_role_grant`. Probe re-run (dev
  profile, nightly + Cranelift per `run.md`): per-item ×8 41.1 ms vs coarse desk
  3.5 ms (11.9×, matches B0's ~10.5×); release re-run pending (LLVM rebuild
  slow, was aborted).
- **Authz B0 — read-gate baseline benchmark**: new probe
  `logic/probe_001_read_gate_assembly_baseline.rs` (wired into `harness.rs`)
  measures the marginal cost B1 adds to hot read paths. Release means:
  `assemble_principal` admin (27 grants) 465 µs / member 60 µs; single-resource
  `authorize` Article::Read 180 µs, Version-chain 230 µs, Comment-chain 297 µs,
  coarse `Virtual::"read"` desk 138 µs; session-only `read_article` body 85 µs.
  Per-item gating a collection page (8) = 1.44 ms vs 138 µs coarse (~10.5×) ⇒
  B1 must gate collection reads once against the coarse desk, not per item.
  Probe kept as the before/after instrument; re-run after B1.
- **Authz A6 — documentation**: wrote `document/authz.md` — the stable
  layered-model record (session/PoW → Cedar authorize → one-time token
  binding), read gating today (session-only) + the Phase B hook locations, and
  the D1/D3/D7 decision records. Put in `document/` because `README.md` §5 is
  another agent's uncommitted file; plan notes a later agent may fold it into
  README §5. No code changed. Phase A complete.
- **Authz A5 — vocabulary single source + policy 6 deleted (committed 2b1f02c)**:
  `SCHEMA` is now production (`infrastructure/cedar.rs`) with a new
  `schema_actions()` helper (parses the schema, returns sorted action names).
  `seed.rs` derives every permission node and the admin grant loop from
  `schema_actions()`; `ALL_PERMISSIONS` deleted from `repository/role.rs`; the
  `PERMISSION_*` constants production no longer uses became `#[cfg(test)]` and
  the drift test now compares schema vs `permission_vocabulary()`. Policy 6
  (admin override permit) deleted in the same slice as the schema-derived admin
  grants — admin power moved from rule to data with no gap; D7's forbid
  protects the admin role. `admin_role_allows_everything` split into
  `admin_holding_a_grant_is_allowed` + `admin_without_a_grant_is_denied`; new
  `every_schema_action_is_seeded_as_a_permission_and_granted_to_admin`. Red
  evidence: restoring policy 6 failed `admin_without_a_grant_is_denied`
  (grantless admin allowed); a schema-only `RED::Probe` action failed the seed
  test under a constants-driven seed (propagates only with the schema-driven
  seed). Back 433, common 109, clippy 0, frontend trunk build clean.
- **Authz A4 — Role::Revoke protection in policy (committed 0d3e7de)**:
  `Role::Revoke` joins the vocabulary (27 actions, seeded to admin).
  `policy.cedar` adds `forbid(principal, action == Role::Revoke, resource ==
  Role::"admin")` so the admin override can't undo the admin role (D7-b; forbid
  beats permit). `Resource::Role` assembles `Role::"..."`. `update_role`
  authorizes removals against `Role::Revoke` on the target role, additions keep
  `Role::Manage` on admin-console. The Rust required-role guard now covers
  recycler/member only so the forbid is not shadowed. Red = admin-revoke got
  400 vs 403 target; `Resource::Role` did not compile. Back 431, common 109,
  clippy 0, frontend trunk build clean.
- **Authz A3 — System → Virtual + single create entry (committed 9a92e1e)**:
  the synthetic entity is `Virtual` (D4): `article-create`, `comment-create`
  and `admin-console` desks. `schema.cedar` declares `entity Virtual`;
  `Article::Create`/`Comment::Create` apply to `[Virtual]` (D6);
  `Version::Create` stays `[Article]`. `Resource::System` → `Resource::Virtual`
  with `virtual_uid` assembly. `authorize_create` deleted; creates route through
  the single `authorize` entry against `Virtual::"article-create"`/
  `"comment-create"`. Red = rewritten tests failed to compile against the old
  enum. Back 428, common 109, clippy 0, frontend trunk build clean.
- **Authz A2 — policy-schema action cross-check (committed 208e94c)**:
  `every_action_referenced_by_policy_exists_in_the_schema` scans every
  `Action::"..."` literal in the policy source and fails if one is missing from
  the schema (red shown by temporarily removing `Article::Read`). Renamed the
  stale `..._twenty_three_seeded_actions` test to `schema_actions_equal_the_seed_vocabulary`
  (count read from schema, not hardcoded); fixed the `schema.cedar` header
  comment (23 → 26 actions). Back 427, common 109, clippy 0, frontend trunk
  build clean.
- **Authz A1 — scope axis removed (committed 8698ecc)**: roles are pure user
  sets; tags stay article content metadata only. `policy.cedar` rule 3 is
  `principal in action`; `scopes`/`global_role`/`required_scopes` and the `Tag`
  entity deleted from `schema.cedar`; assembly no longer reads
  `EDGE_ROLE_APPLY_TAG` or builds scopes; `RoleUpdateRequest.tags` and
  `RoleView`/`RoleListItem.scopes` dropped from the API (frontend had no
  references). Red observed first on `scope_free_policy_allows_create_on_the_virtual_desk`
  (probe) and `role_permission_grants_via_principal_in_action`. Back 426 (3
  obsolete scope tests deleted), common 109, clippy 0 warnings, frontend trunk
  build clean.
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
- **Frontend URL-aligned comment UI (committed bd514fd, 6065d69, 8b66dc6)**:
  comment delete page unified to three radio circles (transfer/soft/hard) + one
  delete button; `comment.rs` split by URL into `comment/index.rs`
  (`/comment` list), `comment/detail.rs` (`/comment/:cid`), `comment/delete.rs`
  (`/comment/:cid/delete`) with a shared `CommentViewContext`; the `*comment_path`
  wildcard route now renders `CommentSection` directly while `VersionDetail` owns
  only `/version/:version_id` (declaration-order matching verified equivalent).
  Backend `get_document` loop hoisted out of per-hit lock/alloc (cba939b). Frontend
  69 tests pass, clippy clean, trunk build clean.

## Next

- Commit the uncommitted slices (one commit each, clean tree).
- Authz: A1–A6 + B0 + B1 done (B1 `295ff6e`). Next: B2 (`read_user` self-view
  to Cedar — build a `User` resource entity, `resource.owner == principal` for
  self-view, admin rides the `User::Read` grant; drop the
  `target_id == actor_id` bypass in `logic/user.rs`), then B3 (central
  operation→action inventory + router coverage test). Open follow-up: release
  re-run of `probe_001` (B1 numbers so far are dev-profile).
- Perf: P2, P3, P5, search-ORDER-BY closed. P1 rejected (highlight behavior);
  P6 non-problem (O(R)); P4 accepted (inherent). Open: total/cursor on list endpoints
  + search total.