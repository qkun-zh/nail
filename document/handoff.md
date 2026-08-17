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
- Current sole pending item: **Permission System Overhaul** (see below,
  ownership unclaimed).

## Remaining risks (inherited from the completed task, for reference)

Coverage capped at 89.10%; the uncovered remainder are all non-user-input paths
(require real SMTP/server/mock, or DB-fault/race defense branches).

————————————————————————————————————————————————————————————————

# Task: Permission System Overhaul

**Ownership**: claimed by the agent receiving this handoff. **Status**: not started.
This is this task's exclusive area; others must not modify it.

## Decisions (final, do not change)

1. `Restore` → `Undelete::Soft`
2. Remove `User::Delete::Transfer`, keep `Version::Delete::Transfer`
3. Split `Role::Manage` into 6 permissions (Create/Read/Update/Delete/Grant/Revoke)
4. Virtual is used only for Create operations (no instance); operations with an
   instance (User/Role) use the concrete resource type
5. User supports soft delete
6. Permission count 32 (was 27): Article=7, Version=7, Comment=7, User=5, Role=6

## Stage A — Cedar authorization layer

- **Slice 1** — schema.cedar permission rename/add/remove
  - `Article/Version/Comment::Restore` → `Undelete::Soft`
  - Remove `User::Delete::Transfer`, add `User::Delete::Soft`, `User::Create`
  - Remove `Role::Manage`, add 6 fine-grained permissions
    (Create/Read/Update/Delete/Grant/Revoke)
- **Slice 2** — schema.cedar resource-type normalization
  - Keep Virtual resource for Create actions (Article/Comment/User/Role Create)
  - `Version::Create` resource is Article (instance exists)
  - Change instance operations (User::Read/Update/Delete,
    Role::Read/Update/Delete/Grant/Revoke) to concrete types User/Role
- **Slice 3** — policy.cedar
  - Update owner bypass (Soft replaces Transfer)
  - Policy 4 uses action-set matching (remove `Virtual::"admin-console"` hardcode)
  - Update Policy 5 recycler restrictions
- **Slice 4** — build.rs
  - Update the test_only list

## Stage B — Backend implementation layer

- **Slice 1** — repository
  - role.rs permission constants auto-generated (add User::Create/Soft,
    remove User::Transfer, Role's 6)
  - delete.rs add `soft_delete_user`
  - authorization.rs resource assembly update: Role/User use concrete
    resource, Virtual only for Create
- **Slice 2** — logic
  - Change three restore → undelete_soft (article/version/comment)
  - user.rs remove transfer, add soft delete, add create authorization
  - role.rs use 6 fine-grained permissions
- **Slice 3** — interface
  - router route rename (restore → undelete-soft)
  - Rename each handler + update permission checks

## Stage C — Operations and tests

- **Slice 1** — operations.rs
  - ROUTE_*_RESTORE → UNDELETE_SOFT
  - ROLE_* permission mapping update
- **Slice 2** — tests
  - Update tests using old permission/route names
  - Add `User::Delete::Soft` tests
  - All tests pass

## Stage D — Full explicit authorization hardening

- **Slice 1** — Fix Virtual abuse (User/Role use concrete resource, not Virtual)
  - Check all User authorization calls pass concrete User resource
  - Check all Role authorization calls pass concrete Role resource
- **Slice 2** — policy completeness
  - Full explicit authorization verification (no implicit-permission holes)
  - Verify admin role policy retained
- **Slice 3** — verification
  - `cargo fmt` / `cargo clippy` (zero warnings)
  - `cargo test` (all green)
  - `trunk build` (frontend)
