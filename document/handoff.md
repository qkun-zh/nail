# Handoff

## Task organization rules (mandatory for every handoff write/update)

1. Every task must be decomposed into a three-level hierarchy, ordered by size:
   **task → stage → slice** (task is the largest unit, stage is intermediate,
   slice is the smallest unit).
   - task is numbered with Roman numerals (e.g. `I.`, `II.`)
   - stage is numbered with capital letters (e.g. `A.`, `B.`)
   - slice is numbered with Arabic numerals (e.g. `1.`, `2.`)
2. A slice, once complete, must be promptly removed from the handoff to prevent
   entropy explosion — keep only incomplete and in-progress entries.
3. Each task must have a clear boundary in the handoff (partitioned by task,
   ownership labeled) to prevent confusion and interference.
4. Do not modify, delete, or interfere with tasks not owned by you; changing
   another's task requires explicit permission.
5. The entire document must be written in English.
6. Each agent's workspace must be separated by a divider of exactly 64
   em-dashes (`—`).

## Current state

- Task "Fix 10 code quality defects": **completed and cleared** (see commits
  03d1c7c..2707dd8). Back coverage 89.10%, all three crates fmt/clippy clean,
  back 499 tests and front 69 tests green.
- Current sole pending item: **Permission System Overhaul** (see below).

## Remaining risks (inherited from the completed task, for reference)

Coverage capped at 89.10%; the uncovered remainder are all non-user-input paths
(require real SMTP/server/mock, or DB-fault/race defense branches).

————————————————————————————————————————————————————————————————

# Task: Permission System Overhaul

**Ownership**: this agent (permission overhaul). **Status**: in progress.
This is this task's exclusive area; others must not modify it.

## Decisions (final, do not change)

1. `Restore` → `Undelete::Soft` (all of Article/Version/Comment/User)
2. Transfer exists only for Article/User/Comment. Article/Comment transfer:
   move the target and its subtree to the recycler. User transfer: move the
   user's subtree (excluding the user node) to the recycler, then delete the
   user. Remove `Version::Delete::Transfer`.
3. Split `Role::Manage` into 6 permissions (Create/Read/Update/Delete/Grant/Revoke)
4. Virtual is used only for Create operations (no instance); operations with an
   instance (User/Role) use the concrete resource type
5. User supports soft delete; self-service deregistration (email-confirmed)
   picks either `soft` or `transfer` mode
6. Permission count 33 (was 27): Article=7, Version=6, Comment=7, User=7, Role=6
7. `User::Create` is declared in schema, enforced via a Cedar check at
   registration, but its policy is permit-all (always allows; conditions can
   be added later without code changes)
8. **Nothing is implicitly permitted. Every operation — including
   self-service deregistration (transfer/soft) and registration — calls
   `authorize()` explicitly; no operation may bypass the Cedar check.**
   Explicit rules may be conditional (e.g. `resource.owner == principal`,
   `principal == resource`); conditional rules ARE explicit authorization.
9. **No implicit grants. All decisions come from explicit conditional rules
    in policy.cedar + explicit `authorize()` calls.** Owner of own content:
    Read/Update/Version::Create/Delete::Soft/Delete::Transfer (Version has no
    Transfer). Owner never has Hard delete or Undelete — those are admin-only.
    User self-view/self-update via `principal == resource`.
10. **Recycler mounts content only — it holds no management permissions.**
    Recycle-bin management (hard delete / undelete of recycled content) is
    admin-only (admin role holds every permission). Recycler transfer forbid
    stays.

## Task I — Cedar authorization layer

- **Stage A** — schema.cedar permission rename/add/remove
  - Slice 1. `Article/Version/Comment/User::Restore` → `Undelete::Soft`
  - Slice 2. Remove `Version::Delete::Transfer`; keep `User::Delete::Transfer`;
    add `User::Delete::Soft`, `User::Create`, `User::Undelete::Soft`
  - Slice 3. Remove `Role::Manage`, add 6 fine-grained permissions
    (Create/Read/Update/Delete/Grant/Revoke)
- **Stage B** — schema.cedar resource-type normalization
  - Slice 1. Keep Virtual resource for Create actions (Article/Comment/User/Role Create)
  - Slice 2. `Version::Create` resource is Article (instance exists)
  - Slice 3. Change instance operations (User::Read/Update/Delete::Hard/Soft/
    Transfer/Undelete::Soft, Role::Read/Update/Delete/Grant/Revoke) to concrete
    types User/Role
- **Stage C** — policy.cedar (all rules explicit; nothing implicitly allowed)
  - Slice 1. Owner-conditional rules for content operations on own content
    (Article/Version/Comment Read/Update, Article/Comment Delete::Soft/
    Delete::Transfer, Version::Delete::Soft, Version::Create — no Hard delete,
    no Undelete: those are admin-only)
  - Slice 2. Self rules: User::Read/User::Update on self (`principal == resource`)
  - Slice 3. Role-grant rule + admin-console rule replaced by concrete-resource
    matching for User/Role instance operations (no Virtual hardcode)
  - Slice 4. Update Policy 5 recycler restrictions (drop Version transfer);
    recycler has no grants
  - Slice 5. Add permit-all policy for `User::Create` (conditions later)
  - Slice 6. `forbid` admin-role revocation stays
- **Stage D** — build.rs
  - Slice 1. Clear the test_only list (Version transfer dropped; User
    Soft/Transfer now runtime-used via self-service authorize)

## Task II — Backend implementation layer

- **Stage A** — repository
  - Slice 1. role.rs permission constants auto-generated (add User::Create/Soft/
    Undelete, keep User::Transfer, remove Version::Transfer, Role's 6)
  - Slice 2. delete.rs add `soft_delete_user` and `undelete_soft_user`
  - Slice 3. authorization.rs resource assembly update: Role/User use concrete
    resource, Virtual only for Create; anonymous principal support for
    registration check
- **Stage B** — logic
  - Slice 1. Change three restore → undelete_soft (article/version/comment)
  - Slice 2. user.rs: keep transfer (mode `transfer`), add soft delete (mode
    `soft`), add undelete_soft, add explicit `authorize()` to self-service
    deregistration (transfer and soft) and to create (permit-all)
  - Slice 3. role.rs use 6 fine-grained permissions
- **Stage C** — interface
  - Slice 1. router route rename (restore → undelete-soft)
  - Slice 2. Rename each handler + update permission checks

## Task III — Operations and tests

- **Stage A** — operations.rs
  - Slice 1. ROUTE_*_RESTORE → UNDELETE_SOFT
  - Slice 2. ROLE_* permission mapping update
- **Stage B** — tests
  - Slice 1. Update tests using old permission/route names
  - Slice 2. Add `User::Delete::Soft`, `User::Undelete::Soft` and `User::Create`
    tests
  - Slice 3. All tests pass

## Task IV — Full explicit authorization hardening

- **Stage A** — Fix Virtual abuse (User/Role use concrete resource, not Virtual)
  - Slice 1. Check all User authorization calls pass concrete User resource
  - Slice 2. Check all Role authorization calls pass concrete Role resource
- **Stage B** — no implicit permission holes
  - Slice 1. Verify every operation path calls `authorize()` (no bypasses,
    including self-service deregistration and registration)
  - Slice 2. Verify no policy grants anything without an explicit rule
    (deny-by-default; owner/self rules are explicit and conditional)
- **Stage C** — verification
  - Slice 1. `cargo fmt` / `cargo clippy` (zero warnings)
  - Slice 2. `cargo test` (all green)
  - Slice 3. `trunk build` (frontend)