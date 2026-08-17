# 003 — Permission system overhaul

## 1. Requirement

Refactor the authorization model in `code/back` from 27 to 33 permissions:
explicit rules only, deny-by-default, no implicit grants anywhere.

| # | R | Acceptance | Status |
|---|---|---|---|
| 1 | `Restore` → `Undelete::Soft` (Article/Version/Comment/User) | schema has 4 Undelete::Soft, no Restore | pending |
| 2 | Remove `Version::Delete::Transfer`; keep `User::Delete::Transfer`; add `User::Delete::Soft`, `User::Create`, `User::Undelete::Soft` | schema action set == 33 | pending |
| 3 | `Role::Manage` → Create/Read/Update/Delete/Grant/Revoke | 6 Role actions | pending |
| 4 | Virtual only for Create ops; instance ops use concrete User/Role | schema appliesTo concrete | pending |
| 5 | User soft delete; self-service deregistration (email-confirmed) picks `soft` or `transfer` | `soft_delete_user` + `undelete_soft_user` | pending |
| 6 | `User::Create` declared, cedar-checked, permit-all policy | authorize call at registration | pending |
| 7 | Every operation calls `authorize()`; no bypass; explicit conditional rules (owner/self) are fine | no implicit permissions | pending |
| 8 | Recycler: mount only, zero grants; recycle-bin management admin-only | policy + seed | pending |

## 2. Scope

In scope: `code/back` schema.cedar, policy.cedar, build.rs, repository
(delete/role/authorization), logic (user/role/article/version/comment/
operations/authorize), interface (router/user/role), `test/unit/back/**`.
Out of scope: `code/front` (no restore/role route references; behavior
unchanged), `code/common` (DeleteMode/TokenPurpose already exist).

## 3. Design decisions

- D1 — **Explicit model**: every op calls `authorize()` (incl. self-service
  deregistration transfer/soft and registration). Conditional rules
  (`resource.owner == principal`, `principal == resource`) are explicit and
  allowed; deny-by-default.
- D2 — **Owner rules cover own-content operations**: Read/Update/
  Version::Create/Delete::Soft/Delete::Transfer (Article/Comment), Delete::Soft
  (Version). Owner never gets Hard delete or Undelete — those are admin-only.
- D3 — **Self-service deregistration**: `User::Delete::Soft` / `User::Delete::Transfer`
  authorized on `Resource::User(actor)` (self), approved by a
  `principal == resource` conditional rule, then email-token verified (both
  layers required).
- D4 — **User::Create**: schema action on Virtual; policy 7
  `permit (principal, action == Action::"User::Create", resource == Virtual::"user-create");`
  (permit-all, no when) — future conditions slot. Call site: `logic/user.rs`
  `create_user`, using a fixed synthetic principal `User::"anonymous"` via a
  new `authorize_anonymous` helper (no DB assembly).
- D5 — **User::Read self-view** stays as explicit `principal == resource`
  rule; admin reads others via role grant.
- D6 — **Recycler**: zero seed grants; forbid-rule against transfer stays
  (admin moderation hard-delete only).
- D7 — `admin_console()` Virtual helper removed; Role/User ops pass concrete
  `Resource::Role(name)` / `Resource::User(id)`.
- D8 — build.rs test_only list cleared (both User transfer/soft now
  runtime-used).

## 4. Slice breakdown

| Slice | Goal | Files | Red | Green | Exit | Status |
|---|---|---|---|---|---|---|
| A1 | schema 27→33, renames, Version transfer drop | schema.cedar | probe_004 action-count | actions==33 | cargo test | pending |
| A2 | resource-type normalization (Virtual→concrete) | schema.cedar | cedar.rs inventory test | passes | cargo test | pending |
| A3 | policy.cedar rewrite (owner/self/grant/permit-all/forbids) | policy.cedar | cedar.rs validation | passes | cargo test | pending |
| A4 | build.rs test_only cleared | build.rs | cargo check | compiles | cargo check | pending |
| B1 | permission_vocabulary 27→33 | repository/role.rs | role.rs test | passes | cargo test | pending |
| B2 | soft_delete_user + undelete_soft_user | repository/delete.rs | delete.rs test | passes | cargo test | pending |
| B3 | Resource assembly for concrete User/Role | authorization.rs | authorize.rs test | passes | cargo test | pending |
| C1 | article/version/comment restore→undelete_soft | logic/{article,version,comment}.rs | http tests | passes | cargo test | pending |
| C2 | user.rs: soft/undelete/create authz/self-service authz | logic/user.rs | user http tests | passes | cargo test | pending |
| C3 | role.rs 6 fine-grained permissions | logic/role.rs | role http tests | passes | cargo test | pending |
| C4 | interface rename + new user route | interface/{router,user,role}.rs | router test | passes | cargo test | pending |
| D1 | ROUTE_ACTIONS update | logic/operations.rs | operations test | passes | cargo test | pending |
| D2 | tests renamed/added (soft/undelete/create) | test/unit/back/** | new tests fail | all green | cargo test | pending |
| E1 | virtual-abuse check, no-bypass audit | logic/** | audit notes | none found | cargo test | pending |
| E2 | final gate | — | — | fmt+clippy+test+trunk | full gate | pending |

## 5. Open unknowns

- Cedar permit-all policy for `User::Create` on Virtual with synthetic
  anonymous principal: source = cedar-policy crate (policy syntax),
  probe_004 validates schema+policy + decide() returns Allow — probe resolves.
- `principal == resource` self-rule for User::Delete::Soft/Transfer:
  source = current policy 1b pattern; probe_004 covers.
- Whether `authorize` requires an existing principal DB row: source =
  `assemble_principal` (DB-free path needed for anonymous) — new
  `authorize_anonymous` bypasses assembly; probe covers.

## 6. Verification plan

| Dimension | How verified |
|---|---|
| Correctness | full back test suite (499+new) after each slice |
| Behavior change | R1–R8 acceptance table; probe_004 action-set equality |
| Time/Space | N/A — same code paths, no new allocation classes |
| Coverage | unchanged baseline (no new infra paths) |

## 7. Risks

- Policy 1 rewrite may break owner story tests (FR-20/FR-21) — expected;
  tests updated to admin for delete-class ops.
- Strict Cedar validation may reject permit-all/self-rules — probe first,
  then finalize policy text.
- 33-constant vocabulary touches many tests — D2 slice absorbs all renames.
- Mitigation: per-slice gates + single-slice commits; rollback = revert commit.

## 8. Constraints

- No `unwrap`/`expect`; no hand-edited Cargo.lock; English only.
- Don't touch front/common; don't change route literals except
  restore→undelete-soft and the new user undelete route.
- One commit per slice on clean tree; never discard work.

## 9. Questions

- Resolved with user: Q1/Q2 explicit model (conditional rules allowed),
  Q3 recycler zero-grant, Q4 no authorize bypass anywhere, DP1 self-service
  soft|transfer, DP2 full User::Undelete::Soft chain, DP3 permit-all
  User::Create. None remaining.

## Change log

- 2026-08-17: created; decisions D1–D8 per user answers to Q1–Q4/DP1–DP3.