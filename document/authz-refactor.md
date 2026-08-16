# Authorization Refactor Plan

Status: **adopted by user — A0 evidence collected (probes green), decisions
D1–D7 recorded, execution started at A1**. Per `workflow.md` 5.5, this file is
the tracking point: update it on every plan change, mark status on adoption.

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

#### A3 — Unify create authorization (O2)
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

#### A4 — Role revocation protection in policy (O5, D7-b)
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

#### A5 — Vocabulary single source + delete policy 6 (O4, D3)
**Changes**:
- Make `SCHEMA` non-`#[cfg(test)]` (`infrastructure/cedar.rs:6`).
- `seed.rs`: seed permission nodes from parsed schema actions; keep the admin
  grant loop but derive it from schema (every action → admin); delete
  `ALL_PERMISSIONS` (`repository/role.rs:39`); keep `PERMISSION_*` constants
  (compile-time call-site vocabulary) with the A2 test guarding drift. The A0
  probe `schema_actions_equal_seed_vocabulary_and_parse_as_uids` must stop
  referencing `ALL_PERMISSIONS` (compare schema actions vs the `PERMISSION_*`
  constants instead).
- Delete policy 6 (`policy.cedar:57-58`) — only after the schema-derived admin
  grants land in the same slice, so admin's power moves from the rule to the
  data without a gap. D7's forbid still protects the admin role.
**Red test**: adding an action to `schema.cedar` alone propagates to a seeded
permission node and to the admin grant; an admin without the grant is denied.
**Exit**: `ALL_PERMISSIONS` deleted; seeds derive from schema; no policy grants
power to a role (admin's power is data, not a permit); cross-check test green.

#### A6 — Documentation (O6)
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

### Phase B — Read enforcement (O1, O5)

#### B0 — Read-gate benchmark (evidence before any code)
**Goal**: measure per-authorize assembly cost (principal graph read + entity
build) on hot paths (`/article/read`, `/article/{id}/read`) before and after
adding the gate. Source + probe per `workflow.md`; decide caching strategy only
on numbers.

#### B1 — Read enforcement (O1, R4 strict form)
**Changes**:
- Thread `actor_id` through read logic (`logic/article.rs:89`,
  `logic/search.rs:13`, `logic/version.rs:131,161`, `logic/comment.rs:67,91,104`)
  and their interface handlers.
- Grant `Article::Read`/`Version::Read`/`Comment::Read` to the member role in
  `seed.rs` (D5) alongside the existing create grants; the PDF download path
  (`logic/download.rs:19`) already authorizes `Version::Read` and passes via this
  grant once policy 2 is removed.
- Single-resource reads authorize against the resource (`Article::Read` /
  `Version::Read` / `Comment::Read`); collection reads authorize once against the
  coarse `Virtual::"read"` desk (principal assembly only, no per-item assembly).
- Fail-closed risk: wiring error → 403 on reads; land atomically per route group
  with red tests per route.
**Exit**: no session-only read gate remains; member reads succeed; non-member
denied.

#### B2 — `read_user` self-view to Cedar (O5)
**Changes**:
- Build a `User` resource entity (`owner` = target); policy: `resource.owner ==
  principal` (self view); admin's view of other users rides the admin role's
  `User::Read` grant through the generic rule (policy 6 is gone, so no
  `Role::"admin"` clause is needed). Remove the `target_id == actor_id` bypass
  (`logic/user.rs:73`).

#### B3 — Central operation→action inventory + router coverage test (O1, O2)
**Changes**:
- One table (operation → action string) consumed by handlers; a test walks every
  route in `interface/router.rs` and asserts a matching entry.

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
