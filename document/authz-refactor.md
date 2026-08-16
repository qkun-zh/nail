# Authorization Refactor Plan

Status: **adopted by user — A0 evidence collected (probes green), decisions
D1–D7 recorded, execution started at A1**. Per `workflow.md` 5.5, this file is
the tracking point: update it on every plan change, mark status on adoption.

Phase B4 (**codegen single-source + policy validation**, user-approved
"修复这三个吧，同时加上测试保证不会修错") added below; B4.0 evidence collected,
B4.1 done (see B4.1).

## 1. Goal

Make the authorization model truthful: what the code claims (policy + schema) is
exactly what runs. Concretely — one enforcement entry, one action vocabulary, no
scope axis, reads enforced, and every rule written in `policy.cedar` instead of
Rust — while preserving today's behavior except for the user-approved deltas
(D1 scope removal, D2 read gates) and with no hot-path regression.

**Definition of done** (all checkable by test or grep):

- Every route authorizes through the single `authorize(state, actor, action,
  resource)` entry; `authorize_create` is deleted; creates are judged against
  real or `Virtual` resources (O2).
- `schema.cedar` is the single action vocabulary: seeds derive from it at
  runtime, `ALL_PERMISSIONS` is deleted, and a test proves every policy-
  referenced action exists in schema (O4).
- No scope machinery remains — no `scopes`/`global_role`/`required_scopes`/
  tag-as-scope; roles are pure sets of users (O3).
- Every read route calls `decide`; no session-only gates remain (O1).
- No policy decision is duplicated in Rust: `read_user` self-view and admin
  revocation protection live in policy, and the shadowing Rust guard is removed
  (O5).
- The layered model (session/PoW → Cedar → one-time token binding) is
  documented (O6).
- Green: full suite, `cargo fmt`, `cargo clippy` 0 warnings; no hot-path
  regression per B0.

## 2. Problems today

| # | Problem | Impact |
| --- | --- | --- |
| P1 | Declared ≠ enforced: `policy.cedar` rule 2 ("any authenticated principal may read") has no enforcement point — read routes (`read_article`, `search_articles`, `read_version(s)`, `read_comments`/`read_comment`/`read_comment_children`) are gated only by the `Principal` session extractor, never `decide`. | Reads are open to any session; policy lies. |
| P2 | Create hack: `authorize_create` fabricates a synthetic `Article::"__create__"` resource (`logic/authorize.rs:50`) with no data backing; `Comment::Create` is judged against an Article entity (schema says appliesTo `[Version, Comment]`). | Two enforcement paths; creates judged against fake resources. |
| P3 | Scope-axis conflation: `Tag` is overloaded — content tag (`article_apply_tag`) and role "scope" (`role_apply_tag`) share one node type, one name space, one Cedar `Tag::"name"` UID. Editing an article's tags silently changes which roles' permissions apply to it. Hidden escalation: a role whose `scopes` list empties becomes `global_role` (`authorization.rs:104`); future orphan-tag cleanup that only counts `article_apply_tag` edges would delete a role's scope tag and silently widen that role's power. | Confused semantics + privilege-escalation hazard. |
| P4 | Vocabulary duplicated ~6 places: `schema.cedar`, `role.rs` `PERMISSION_*` + `ALL_PERMISSIONS`, seed permission nodes, policy action lists, call sites. Only one cross-check test exists; policy-referenced actions are unchecked. `SCHEMA` is `#[cfg(test)]` only — the claimed single source is never loaded at runtime. | Permission churn touches many files; missed places are silent. |
| P5 | Rules leaking into code: `read_user` self-view bypass (`logic/user.rs:73`, `target_id == actor_id`) is a policy decision written in Rust. | Policy intent is invisible; drift risk. |
| P6 | Schema does not describe runtime: runtime builds Role→Action, Version→Article, Comment→Version `in`-chains; schema declares only `User in [Role]`. No schema validation at runtime (`Entities::from_entities(entities, None)` in `infrastructure/cedar.rs:46`). | Runtime graph drifts from declared model undetected. |

## 3. Objectives & acceptance

Each objective is a crisp target with a checkable acceptance and the slices that
deliver it.

### O1 — Reads enforced (P1)
Read routes get real Cedar gates; the session-only gap closes.
- **Acceptance**: every read route calls `decide`; member reads succeed via
  explicit grants (D5); non-members denied; no session-only read gate remains.
- **Delivered by**: B1, B3.

### O2 — Single enforcement entry (P2)
Exactly one `authorize(state, actor, action, resource)` path.
- **Acceptance**: `authorize_create` deleted; `Article::Create` /
  `Comment::Create` judged against `Virtual` resources; `Version::Create`
  judged against its parent Article.
- **Delivered by**: A3.

### O3 — No scope axis (P3, D1)
Roles are pure sets of users; tags are content metadata only.
- **Acceptance**: no `scopes`/`global_role`/`required_scopes`/`EDGE_ROLE_APPLY_TAG`
  anywhere; `Tag` gone from schema; the escalation hazard is gone.
- **Delivered by**: A1.

### O4 — Single vocabulary source (P4)
`schema.cedar` is the enforced source of action names, loaded at runtime.
- **Acceptance**: permission nodes + admin grants seed from parsed schema;
  `ALL_PERMISSIONS` deleted; a test proves policy-referenced actions exist in
  schema and match `PERMISSION_*` constants; adding/removing an action fails
  loud.
- **Delivered by**: A2, A5.

### O5 — Rules in policy, not Rust (P5 + D7)
Policy decisions stop being reimplemented in code.
- **Acceptance**: `read_user` self-view decided by `resource.owner == principal`;
  admin revocation blocked by a policy forbid, not a Rust guard.
- **Delivered by**: A4, B2.

### O6 — Layered model documented (R6)
认证 (session/PoW) → 授权 (Cedar) → 防滥用 (one-time token binding) written down
as a model, not a smell.
- **Acceptance**: layered model + how reads are gated today (and where the
  Phase B hook lands) documented; decisions recorded.
- **Delivered by**: A6.

## 4. Design decisions (all resolved)

- D1 **Scope axis removed**: roles are pure sets of users; the scope mechanism
  (`role_apply_tag`, `scopes`, `global_role`, `required_scopes`, `Tag` in
  schema) is deleted; tags stay content metadata. Consequence: create is
  principal-level only; the conflation and escalation hazard disappear. **User
  decision.**
- D2 **Phase B in scope**: read-path enforcement (B1–B3) is in scope. Execution
  order: Phase A → Phase B (B0 benchmark first; B0 numbers gate B1 as a
  fail-closed/perf safety check). **User decided.**
- D3 **No magic admin rule**: policy 6 (`principal in Role::"admin"` override) is
  deleted — admin power is explicit data grants (every schema action, same as
  member's create/read grants), so a new action is granted to admin at next
  startup via the same seed loop. C-wart dissolves (admin shows a full
  permission list). D7 guards accidental lockout. **User chose delete policy 6 +
  explicit full admin grants** (supersedes D3-A and 方案三).
- D4 **`Virtual` resource entity**: the "platform-level operation" resource is
  named `Virtual` (was `System`): `Virtual::"article-create"`,
  `Virtual::"comment-create"`, `Virtual::"admin-console"` — a synthetic entity
  giving Cedar a target for operations with no content node (Cedar requires a
  resource on every request). **User chose `Virtual`.**
- D5 **Member read grants**: `seed.rs` grants `Article::Read`, `Version::Read`,
  `Comment::Read` to the member role alongside the two create grants so B1's read
  gates don't lock out members. **User agreed.**
- D6 **Create targets**: `Article::Create` → `Virtual::"article-create"`;
  `Comment::Create` → `Virtual::"comment-create"` (the "comment desk");
  `Version::Create` stays judged against its parent `Resource::Article` —
  moving it to a Virtual resource would break policy 1's owner bypass, which
  members rely on. **User chose the desk for comments; Version::Create
  unchanged.**
- D7 **Admin revocation protection lives in policy (精确版 b)**: revocation
  (removing permissions or users from a role) is split out of the coarse
  `Role::Manage` into a new `Role::Revoke` action, judged against the **target**
  role as resource:
  `forbid(principal, action == Role::Revoke, resource == Role::"admin")`.
  `Role::Manage` stays for role create/read/update-adds/delete against
  `Virtual::"admin-console"`. The existing Rust guard (`logic/role.rs:146-154`,
  blocks destructive changes to REQUIRED_ROLES) keeps protecting recycler/member,
  but its admin coverage is removed — otherwise the guard would intercept the
  request first and the forbid would be dead code. **User chose the policy
  forbid.**

## 5. Execution plan

Convention per slice: red test → green → gate (`cargo fmt`, `cargo clippy` 0
warnings, `cargo test` in `code/{common,back,front}`) → one commit on a clean
tree. Baseline: back tests green, clippy clean. Slices are atomic and ordered by
dependency.

### Phase A — Core model (O2, O3, O4, O5, O6)

#### A0 — Evidence probes (done, no production code)
**Goal**: de-risk the two Cedar assumptions the design leans on before any code.
- **U1** Missing-attribute semantics: `resource.owner == principal` against an
  attribute-less `Virtual` resource must be a safe Deny (policy not applicable),
  never an error/panic; also proves the A3 create design (`principal in action`
  allows a member holding `Article::Create` against `Virtual::"article-create"`,
  denies a non-holder).
- **U2** `Schema::actions()` usable as the seeding source: parse `SCHEMA` (the
  exact file content) at runtime, enumerate action names, assert they equal
  `ALL_PERMISSIONS` (26) and parse as `Action::"..."` UIDs.

**Evidence** (probe tests in `test/unit/back/infrastructure/cedar_probe.rs`,
wired via `harness.rs`; all green, baseline 429 green, fmt/clippy clean):
- U1 · `missing_attribute_is_a_safe_deny_not_an_error`: reading `resource.owner`
  on an attribute-less `Virtual::"article-create"` → **Deny, never an
  error/panic**.
- U1 · `create_holder_is_allowed_on_the_virtual_desk`: `principal in action`
  allows a member holding `Article::Create` against `Virtual::"article-create"`.
- U1 · `create_non_holder_is_denied_on_the_virtual_desk`: non-holder → Deny.
- U1 · `scope_free_policy_allows_create_on_the_virtual_desk`: with policy 3
  simplified to `principal in action`, a member holding `Article::Create` is
  allowed on the Virtual desk (flipped from the A0 Deny observation in A1).
- U2 · `schema_actions_equal_seed_vocabulary_and_parse_as_uids`: `SCHEMA` parses
  at runtime; **26 actions**; equals `ALL_PERMISSIONS`; every name parses as an
  `Action::"..."` UID via `action_uid` — the A5 seeding source is proven.

#### A1 — Remove the scope axis (O3) — **DONE** (commit `8698ecc`)
**Changes**:
- `schema.cedar`: delete the `scopes`/`global_role` User attributes,
  `required_scopes` on Article/Version/Comment, and the `Tag` entity.
- `policy.cedar`: workhorse rule becomes `principal in action`; delete the
  `global_role || scopes.containsAny(...)` terms.
- Assembly (`repository/authorization.rs`): drop `tag_uid`, `set_expression`
  scope building, `has_global_role`; stop reading `EDGE_ROLE_APPLY_TAG`.
- Graph (`repository/schema.rs`, `repository/role.rs`, `logic/role.rs`): remove
  `apply_tag_to_role`/`remove_tag_from_role`, `EDGE_ROLE_APPLY_TAG`, the `scopes`
  field; keep `article_apply_tag` as content metadata only.
- API (`nail_common`): delete `RoleView.scopes`/`RoleListItem.scopes`
  (`response/role.rs`) and `RoleUpdateRequest.tags` (`request.rs`);
  `interface/role.rs` stops mapping them (frontend has no references).
**Red test**: no test references `global_role`/`scopes`/`required_scopes`; a
role authorizes purely via `principal in action`; the orphan-tag cleanup test
pins that article tags are the only tag usage.
**Exit**: no `EDGE_ROLE_APPLY_TAG`, no scope attributes, no `Tag` in schema;
suite green.
**Evidence (green)**: red observed on `scope_free_policy_allows_create_on_the_virtual_desk`
and `role_permission_grants_via_principal_in_action` before the change; after —
grep clean (no `scopes`/`global_role`/`required_scopes`/`EDGE_ROLE_APPLY_TAG`),
back 426 (was 429: 3 obsolete scope tests deleted), common 109, clippy 0
warnings (back/common/front), frontend `trunk build` clean.

#### A2 — Policy coverage test (O4) — **DONE** (commit `208e94c`)
**Changes**:
- Extend the cross-check: every action referenced by `policy.cedar` exists in
  `schema.cedar` (red test for a removed action).
- Rename the stale test (`..._twenty_three_seeded_actions` → count from schema,
  26 today); fix the stale header comment in `schema.cedar` ("23 actions" → 26).
  A4 adds `Role::Revoke` (27), so the name must not hardcode the count.
**Exit**: renaming/removing an action from policy fails the suite.
**Evidence**: `every_action_referenced_by_policy_exists_in_the_schema` scans the
policy source for `Action::"..."` literals; red shown by temporarily removing
`Article::Read` from `schema.cedar` (test failed) then restoring. Renamed
`schema_actions_equal_the_seed_vocabulary` (reads count from schema). Header
comment fixed. Back 427 (was 426 +1 new test), common 109, clippy 0, frontend
`trunk build` clean.

#### A3 — Unify create authorization (O2) — **DONE** (commit `9a92e1e`)
**Changes**:
- Rename the synthetic entity `System` → `Virtual` (`schema.cedar`,
  `policy.cedar` `System::"admin-console"`, `Resource::System` enum + assembly,
  admin-console call sites) — D4.
- `schema.cedar`: point `Article::Create` appliesTo at `[Virtual]` and
  `Comment::Create` at `[Virtual]`; `Version::Create` stays `[Article]` (D6).
- `policy.cedar`: no new rule needed — after A1 the workhorse rule is
  `principal in action` (resource-agnostic), which covers creates on `Virtual`
  resources. Keep policy 4's `resource == Virtual::"admin-console"` guard.
- Delete `authorize_create` (`logic/authorize.rs:42-66`); call sites
  `logic/article.rs:48`, `logic/comment.rs:32,49` use `authorize` against the
  `Virtual` resources.
**Red test**: create allowed for holders; non-holder denied;
`Resource::Virtual` assembly covers the new uids.
**Exit**: no `authorize_create` remains; create requests route through the
single entry.
**Evidence**: red = the rewritten create tests and
`virtual_desk_assembly_covers_the_create_and_admin_uids` failed to compile
against `Resource::System` (no `Virtual` variant). After: `authorize_create`
deleted; creates route through `authorize` against
`Resource::Virtual("article-create")` / `("comment-create")`; assembly produces
`Virtual::"..."` uids; grep clean (`System::`/`authorize_create` zero hits).
Back 428, common 109, clippy 0, frontend `trunk build` clean.

#### A4 — Role revocation protection in policy (O5, D7-b) — **DONE** (commit `0d3e7de`)
**Changes**:
- Vocabulary: add `Role::Revoke` action (`schema.cedar`); `Role::Manage` stays.
  26 → 27 actions.
- `policy.cedar`: new forbid
  `forbid(principal, action == Role::Revoke, resource == Role::"admin")`.
  `Role::Revoke` is NOT added to policy 4's admin-console list — it is judged
  against the target role resource and governed by the generic `principal in
  action` rule plus the new forbid.
- Assembly (`repository/authorization.rs`): add `Resource::Role(name)` →
  `Role::"..."` entity (no attributes needed; `authorize` provides existence
  check).
- `logic/role.rs` `update_role`: revocations (`permissions_remove`,
  `users_remove`) authorize `Role::Revoke` against `Resource::Role(target)`;
  adds authorize `Role::Manage` as today (both, when the request has both).
- Remove the admin part of the Rust REQUIRED_ROLES guard
  (`logic/role.rs:146-154`); keep it for recycler/member (minus `tags_remove`
  after A1) so the policy forbid is not shadowed by a Rust rejection.
**Red test**: revoking from the admin role is denied (403); revoking from other
roles works for holders; `Resource::Role` assembly covers the uids.
**Exit**: admin revocation blocked by policy; `Role::Revoke` in schema/seed/UI;
suite green.
**Evidence**: red observed two ways — `revoke_from_the_admin_role_is_forbidden`
got 400 (old Rust guard) vs 403 target; `role_resource_assembly_covers_role_uids`
did not compile (no `Resource::Role` variant). After: forbid blocks admin
revocation through the admin override (forbid beats permit); `update_role`
routes removals via `Role::Revoke` on `Resource::Role(target)`, additions keep
`Role::Manage`; guard covers recycler/member only. `Role::Revoke` seeded to
admin via `ALL_PERMISSIONS`. Probe count pinned to 27. Back 431 (was 428 +3
tests), common 109, clippy 0, frontend `trunk build` clean.

#### A5 — Vocabulary single source + delete policy 6 (O4, D3) — **DONE** (commit `2b1f02c`)
**Changes**:
- `SCHEMA` is now production (`infrastructure/cedar.rs`); added
  `schema_actions()` which parses the schema and returns the sorted, deduped
  action names (the single seeding source, A0-proven).
- `seed.rs`: permission nodes and the admin grant loop derive from
  `schema_actions()` (every schema action → permission node → admin role);
  `ALL_PERMISSIONS` deleted from `repository/role.rs`; the `PERMISSION_*`
  constants that production no longer touches became `#[cfg(test)]`, and the
  drift test compares schema actions against `permission_vocabulary()` (the
  same constant list).
- Policy 6 (admin override `permit ... principal in Role::"admin"`) deleted in
  the same slice as the schema-derived admin grants — admin's power moved from
  the rule to the data with no gap. D7's forbid still protects the admin role.
- Tests: `admin_role_allows_everything` split into `admin_holding_a_grant_is_allowed`
  and `admin_without_a_grant_is_denied`; drift test renamed
  `schema_actions_equal_the_permission_constants`; new
  `every_schema_action_is_seeded_as_a_permission_and_granted_to_admin`.
**Red evidence** (captured, then reverted):
- Temporarily restoring policy 6 makes `admin_without_a_grant_is_denied` fail
  (assertion failed: !decide) — the rule permits a grantless admin.
- Temporarily adding `action "RED::Probe"` to `schema.cedar` with a constants-
  driven seed makes the seed test fail (`admin must hold every schema action:
  RED::Probe`) — a schema-only action does not propagate to seed. With the
  schema-driven seed the same mutation propagates (test passes).
**Exit**: `ALL_PERMISSIONS` deleted; seeds derive from schema; no policy grants
power to a role (admin's power is data, not a permit); cross-check test green.
Gate: back 433 / common 109, clippy 0, front `trunk build` clean.

#### A6 — Documentation (O6) — **DONE**
**Changes**:
- Record the layered model (session/PoW → Cedar authorize → one-time token
  binding) in `README.md` §5 or `document/`.
- State explicitly how reads are gated today (session-only) and where the Phase B
  hook will live.
- Record D1 (scope axis removed; roles = sets of users; tags = content metadata
  only) and the escalation hazard it removes.
- Record D3 (admin power = explicit data grants, policy 6 deleted) and D7 (admin
  revocation blocked by a policy forbid; recycler/member kept guarded in Rust).
- Update `document/handoff.md`.
**Done**: wrote `document/authz.md` (layered model, read gating today + Phase B
hook, D1/D3/D7 records). Used the `document/` branch: `README.md` §5 is owned by
the concurrent docs-refactor agent (uncommitted), so the durable record lives in
`document/authz.md` and is linked from the plan; a later agent may fold it into
README §5 once the other agent's edits land. No code changed.

### Phase B — Read enforcement (O1, O5)

#### B0 — Read-gate benchmark (evidence before any code) — **DONE** (probe `probe_001`)
**Goal**: measure per-authorize assembly cost (principal graph read + entity
build) on hot paths (`/article/read`, `/article/{id}/read`) before and after
adding the gate. Source + probe per `workflow.md`; decide caching strategy only
on numbers.
**Baseline (release build, `probe_001_read_gate_assembly_baseline`, kept as the
before/after instrument; B1 re-run in dev profile — see B1)**, mean per call:
- `assemble_principal` admin (27 grants): **465 µs**; member (2 grants): **60 µs**.
- `authorize` `Article::Read` single-resource: **180 µs**; on Version (chain):
  **230 µs**; on Comment (chain): **297 µs**; on `Virtual::"read"` desk: **138 µs**.
- session-only read body `read_article`: **85 µs**.
- Collection page of 8: per-item gate **1.44 ms** vs coarse desk **138 µs** (~10.5×).
**Conclusion**: a single gate adds ~180 µs on the article hot path (read body is
85 µs) — sub-millisecond, acceptable. Per-item gating a page costs ~10.5× the
coarse `Virtual::"read"` desk, so **B1 collection reads must authorize once
against the coarse desk (principal assembly only), not per item**. Admin's
27-grant principal assembly (465 µs) dominates member (60 µs): if admin becomes
hot, cache principal assembly keyed by user; not needed for the B1 default path.

#### B1 — Read enforcement (O1, R4 strict form) — **DONE**
**Changes** (as planned):
- Threaded `actor_id` through the read logic (`logic/article.rs` `read_article`,
  `logic/search.rs` `search_articles`, `logic/version.rs`
  `read_version`/`read_versions`, `logic/comment.rs`
  `read_comments`/`read_comment`/`read_comment_children`) and their interface
  handlers.
- Granted `Article::Read`/`Version::Read`/`Comment::Read` to the member role in
  `seed.rs` (D5) alongside the create grants; the PDF download path
  (`logic/download.rs`) already authorizes `Version::Read` and passes via this
  grant.
- Single-resource reads gate with `authorize_or` against the resource
  (`Article::Read`/`Version::Read`/`Comment::Read`, not-found message wins when
  the resource is absent); collection reads gate once with `authorize` against
  the coarse `Virtual::"read"` desk (principal assembly only, no per-item
  assembly), placed before validation so the gates fail closed.
- Deleted the read-open rule (policy 2); policy numbering kept stable so doc
  cross-references still hold (`authz.md:36`).
- Promoted `PERMISSION_ARTICLE_READ`/`PERMISSION_COMMENT_READ` from test-only to
  production constants in `repository/role.rs`.
**Red→Green**: 8 interface red tests (one per read route: article read/search,
version read/read-versions, comments/comment/comment-children, content read) —
all observed 200 pre-change vs expected 403. Green: **446 back tests**; non-member
reads denied at both interface (403) and logic (`LogicError::forbidden`) level;
member reads succeed via the D5 grant. New logic-level denial tests cover all
seven read functions; the cedar policy test was rewritten from read-open to
grant-based (`read_requires_a_role_grant`).
**Perf**: B1 re-run of `probe_001` (dev profile, nightly + Cranelift per
`run.md` — release re-run would need a full LLVM rebuild): `read_article` gated
body 5.1 ms, single-resource gate 5.1 ms, coarse `Virtual::"read"` desk 3.5 ms,
per-item ×8 41.1 ms vs coarse desk 3.5 ms (**11.9×**, matching B0's ~10.5×
ratio). Absolute values differ from B0's release baseline; the structural
conclusion (one coarse desk gate per collection page, not per item) holds.
**Exit**: met — no session-only read gate remains; member reads succeed; non-member
denied.

#### B2 — `read_user` self-view to Cedar (O5) — **DONE**
**Changes** (as planned):
- Built a `User` resource entity (`owner` = target; schema `entity User in
  [Role] { owner?: User }` — optional so the attr-less principal entity stays
  valid). `User::Read` now applies to a `User` resource (schema) and is judged
  by policy 1 (`resource.owner == principal`, self-view) or by policy 3 (admin
  rides the admin role's `User::Read` grant — no `Role::"admin"` clause needed,
  policy 6 is gone). `User::Read` left policy 4 (admin-console), which now
  covers only `User::Update`/`Delete`/`Role::Manage`.
- Removed the `target_id == actor_id` bypass (`logic/user.rs`): `read_user` gates
  once with `authorize_or` against `Resource::User(target_id)` ("user not found"
  message). Response contract preserved: self-view omits `id`, other-view (admin)
  includes it. `admin_console()` remains for `update_user`/`delete_user`.
- `Resource::User` assembly checks the user node exists (`ResourceNotFound`
  when deleted → 404) and attaches `owner` = target. Duplicate-Uid assembly
  (self-view: principal and resource are the same `User::"<id>"`) merges by
  keeping the resource entity so `resource.owner` is present.
**Red→Green**: 2 red tests — cedar policy test
`user_self_view_allows_anyone_and_other_users_need_a_grant` (self-view was
Denied pre-change) and http test `user_read_self_after_hard_delete_is_not_found`
(401 observed vs 404 target). Green: **448 back tests** — self-view succeeds for
role-less users (owner bypass, no grant needed), admin views of other users stay
allowed via the seeded grant, plain-member views of other users stay 403, and
hard-deleted self-read is 404. fmt/clippy 0, common 109, frontend trunk build
clean.
**Exit**: met — no `read_user` policy decision is duplicated in Rust.

#### B3 — Central operation→action inventory + router coverage test (O1, O2) — **DONE**
**Changes**:
- New `logic/operations.rs`: `ROUTE_ACTIONS`, the one table mapping every route
  path to its Cedar action(s) (empty list = no Cedar gate: the six public /
  session-only routes). Entries use the `PERMISSION_*` constants (O4 single
  vocabulary). Mode-dependent deletes list their real actions: article/comment
  `Hard/Transfer/Soft`, version `Soft/Hard` (Transfer is a bad_request),
  user `Hard` only (the transfer path is PoW-token-gated, not Cedar).
- `build_router` consumes the inventory at boot (per-route debug log); the
  router coverage test is the strict enforcer.
- Tests in `test/unit/back/infrastructure/cedar.rs`:
  `every_route_in_router_has_an_inventory_entry` walks the `.route("...")`
  literals in `interface/router.rs` (via `include_str!`, mirroring
  `policy_action_names`) and asserts every route has an entry, every entry has a
  route, and no route is duplicated; `every_inventory_action_exists_in_the_schema`
  asserts every listed action is declared in `schema.cedar` and matches a
  `PERMISSION_*` constant.
**Red→Green**: table written with `/comment/{id}/restore` omitted → the coverage
test failed naming that route → entry added → green. (Multi-line `.route(`
registrations initially defeated the source parser; fixed to skip to the next
string literal.) Green: **450 back tests** (448 + 2); fmt/clippy 0, common 109,
frontend trunk build clean. New-route workflow is now explicit: add the
`.route()` literal, the table entry (actions), and the handler arm — the test
fails loud if any are missing.
**Exit**: met — every route has an explicit authorization inventory entry and
the inventory can't name an action that isn't a real, seeded permission.

## 6. Risks & mitigations

| Risk | Slice | Mitigation |
| --- | --- | --- |
| Scope removal widens write surface (role with `Article::Update` can now touch any article, not just tag-scoped ones) | A1 | D1 accepted the change; document it in A6; owner/admin bypass still holds |
| Admin lockout if admin role grants are revoked (policy 6 gone) | A4 | D7-b: `forbid(principal, action == Role::Revoke, resource == Role::"admin")` + red test; recycler/member keep the Rust guard |
| Read gate fail-closed outage | B1 | land per route group, red tests each, perf benchmark first |
| Read-path performance regression | B1 | B0 numbers; coarse `Virtual::"read"` gate for collections; caching only on evidence |
| Regression surface (every handler entry) | A1/A3/B1 | strict per-slice red→green→gate; suite already broad |
| Concurrent agents on same tree | all | `AGENTS.md`: disjoint scope, re-read before depending, own commits |
| Half-migrated read gate left in tree | B1 | slice-gated; never commit partial route groups |
| Policy 6 deleted before admin grants land → admin outage | A5 | delete the rule and add the grants in the same slice; red test for grantless admin |

## 7. Verification dimensions (per `workflow.md`)

- **Correctness**: normal + edge cases (owner/admin/recycler/non-holder; create
  before/after; read gates).
- **Behavior change**: before/after diffs must equal the objectives; D1 (scope
  removal) and D2 deltas explicit and user-approved.
- **Time/space**: A1/A3/A4 add no hot-path cost; B1 evidenced by B0 benchmark.
- **Performance**: B0 before/after latency for the two hot routes.

## 8. Adoption gate

Per `workflow.md` 5.5: evidence (A0) presented, decisions D1–D7 recorded, and the
user explicitly adopts this plan before any slice starts.

## 9. Phase B4 — Codegen single-source + policy validation

Status: **adopted by user** ("修复这三个吧，同时加上测试保证不会修错"),
B4.0 evidence collected, B4.1 done, B4.2 done, B4.3 done. Three fixes, each with guard tests so a wrong
"repair" cannot pass:

1. **Fix 2 (point 2) — `PERMISSION_*` constants generated from `schema.cedar`**:
   replace the hand-written mirror in `repository/role.rs` with a `build.rs`
   that parses the schema's `action "..."` lines and emits the constants,
   making schema → constants constructively consistent. Test-only transfer
   constants (`User::Delete::Transfer`, `Version::Delete::Transfer`) keep
   `#[cfg(test)]`.
2. **Fix 1 (point 1) — `ROUTE_*` constants generated from `router.rs`**:
   `build.rs` parses the `.route("...")` literals and emits `ROUTE_<SLUG>`
   constants; `ROUTE_ACTIONS` keys use them, so route → inventory keys are
   constructively consistent. `router.rs` stays the literal source (that is
   where axum registration must read from).
3. **Fix 3 — Cedar `PolicySet::validate(&schema)`** at policy parse time,
   fail-fast at boot. Current policy/schema FAIL strict validation
   (`probe_002`, see B4.1) because `User` doubles as principal (attr-less)
   and resource (carries optional `owner`) — fixed by removing the dual role.

### B4.0 — Evidence probes (done)

- `probe_002_policy_validate` (RED, `test/unit/back/infrastructure/probe_002_policy_validate.rs`,
  wired via `harness.rs`): `Validator::new(schema).validate(&pset,
  ValidationMode::Strict)` on the current `POLICY`/`SCHEMA` fails:
  `for policy policy0, unable to guarantee safety of access to optional
  attribute owner on entity type User`. Source evidence: `cedar-policy`
  4.12.0 `api.rs` — `PolicySet` has no `validate`; validation is
  `Validator::new(schema).validate(&pset, mode)` (`api.rs:1532`),
  `ValidationResult::validation_passed()` (`api.rs:2299`), `ValidationMode`
  defaults to `Strict` (`api.rs:1473`).

### B4.1 — Fix 3: policy validation + root-cause `User` dual-role fix (done)

**Root cause**: `User` doubles as principal (attr-less) and resource (carries
`owner?: User`). The optional attribute exists only to support self-view via
`resource.owner == principal` — but self-view is semantically just
`principal == resource`. So the optional-attribute access (and the earlier
`has()` guard idea) disappears entirely by removing the dual role.

**Changes**:
- `schema.cedar`: `entity User in [Role] { owner?: User };` →
  `entity User in [Role];` (User is attr-less again).
- `policy.cedar` rule 1: drop `Action::"User::Read"` from the action list;
  condition stays `resource.owner == principal` (no `has()` needed — all
  remaining rule-1 resources `Article`/`Version`/`Comment` declare required
  `owner`). New rule 1b: `permit (principal, action ==
  Action::"User::Read", resource) when { principal == resource };`
  (self-view).
- `repository/authorization.rs` `Resource::User`: assemble the resource with
  `Entity::new_no_attrs` (no `owner` attr anymore); existence check → 404 kept.
- `infrastructure/cedar.rs` `policies()`: after `POLICY.parse::<PolicySet>()`,
  run `Validator::new(SCHEMA.parse()?).validate(&pset, Strict)`; on failure
  return an error naming the validation messages. Fail-fast at first use
  (OnceLock, startup).
- `probe_002` promoted → real test `policy_set_validates_against_the_schema`
  in `test/unit/back/infrastructure/cedar.rs`; probe file deleted; self-view
  test updated to attr-less `User` entities.
**Red→Green**: probe was red (see B4.0). After the fix, probe test green.
**Exit**: `policies()` returns an error if policy and schema ever disagree;
policy/schema both pass strict validation; suite green (452 back tests;
fmt/clippy 0, common 109, frontend trunk build clean).

### B4.2 — Fix 2: permission constants via build.rs (done)

**Changes**:
- New `code/back/build.rs`: parses `src/infrastructure/cedar/schema.cedar`
  `action "..."` lines → emits `pub const PERMISSION_<SEGMENTS>: &str = "...";`
  (segments joined by `_`, upper-snake) into `OUT_DIR/permissions.rs`, with
  `#[cfg(test)]` on the two transfer actions (`Version::Delete::Transfer`,
  `User::Delete::Transfer`) that production does not use.
- `repository/role.rs`: the 27 hand-written constants replaced by
  `include!(concat!(env!("OUT_DIR"), "/permissions.rs"));`.
  `permission_vocabulary()` and every `use crate::repository::role::PERMISSION_*`
  call site keep compiling (names unchanged).
- `Cargo.toml`: `build = "build.rs"`; `build.rs` emits
  `cargo:rerun-if-changed=src/infrastructure/cedar/schema.cedar`.
**Guard tests** (prove the generator is right, not just that it ran):
- existing `schema_actions_equal_the_permission_constants` compares the
  generated constants against the parsed schema (now guards generation);
- new `generated_permission_constants_have_expected_names` spot-checks the
  naming rule (segments→upper-snake, `::`→`_`) on representative constants
  and the two transfer constants;
- existing A5 tests (seed derives from schema; drift test) stay green.
**Red→Green**: generator output byte-identical to the replaced constants
(verified in OUT_DIR before wiring `include!`).
**Exit**: no hand-written `PERMISSION_*` remains; adding an action to
`schema.cedar` recompiles constants and (via A5 tests) propagates to seed;
suite green (453 back tests; fmt/clippy 0).

### B4.3 — Fix 1: route constants via build.rs (done)

**Changes**:
- `build.rs` (extended): also parses `src/interface/router.rs`
  `.route("...", ...)` literals → emits `pub const ROUTE_<SLUG>: &str = "...";`
  (slug: strip leading `/`, split on `/`, strip `{`/`}` from segments,
  join `_`, upper-case) into `OUT_DIR/routes.rs`. Emits
  `cargo:rerun-if-changed=src/interface/router.rs`.
- `logic/operations.rs`: adds
  `include!(concat!(env!("OUT_DIR"), "/routes.rs"));` and writes `ROUTE_ACTIONS`
  keys as `ROUTE_*` constants (no more string literals for route paths).
**Guard tests**:
- existing `every_route_in_router_has_an_inventory_entry` (walks router.rs
  literals via `include_str!`) now verifies the generated constants keep
  agreeing with the literal source;
- new `generated_route_constants_match_their_literal_paths` spot-checks five
  `ROUTE_*` constants (including the longest `ARTICLE..VERSION..CONTENT_READ`
  path) against their literal values.
**Red→Green**: generated output byte-identical to the replaced string literals
(verified against the current `ROUTE_ACTIONS` table).
**Exit**: route → inventory-key consistency is constructive; suite green
(454 back tests; fmt/clippy 0, common 109, frontend trunk build clean).

### B4.4 — Gate & docs

**Gate**: `cargo fmt`, `cargo clippy` 0 warnings, full back suite, `common
--lib`, frontend `trunk build`. Update `document/authz-refactor.md` B4 status
and `document/handoff.md`.

## 10. B4 Risks & mitigations

| Risk | Slice | Mitigation |
| --- | --- | --- |
| Cedar strict validation rejects a policy term that is safe at runtime | B4.1 | probe first; root-cause fix removes the dual-role `User` (no `has()` shim); full policy test suite |
| Behavior change from removing `User.owner` | B4.1 | self-view re-expressed as `principal == resource`; `user_self_view...` + full cedar suite prove semantics |
| Generated constants silently disagree with source | B4.2/B4.3 | bidirectional guard tests (constants vs parsed schema / router literals) + rerun-if-changed |
| `include!`/OUT_DIR wiring breaks crate build | B4.2/B4.3 | build.rs + Cargo.toml change lands with its guard tests; gate runs the full suite |
| Changing `policies()` to fail-fast introduces a new boot error path | B4.1 | error carries validation messages; existing boot tests green |
