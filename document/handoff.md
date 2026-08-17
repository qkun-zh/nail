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
- Task **Permission System Overhaul**: **completed** (commits af57930..c146a89,
  nine slices S1–S9). Back 513 tests, front 69 tests, trunk build green,
  fmt/clippy zero warnings across all three crates. See
  `document/exec/003_permission_overhaul.md` for the full record.

## Remaining risks (inherited from the completed task, for reference)

Coverage capped at 89.10%; the uncovered remainder are all non-user-input paths
(require real SMTP/server/mock, or DB-fault/race defense branches).

————————————————————————————————————————————————————————————————

# Task: Permission System Overhaul

**Ownership**: this agent (permission overhaul). **Status**: completed.
This is this task's exclusive area; others must not modify it.

All nine slices S1–S9 are done and committed (af57930..c146a89); the task is
cleared. Record of execution: `document/exec/003_permission_overhaul.md`.

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

## Execution record

All slices across Tasks I–IV (schema, policy, build.rs, repository, logic,
interface, operations, tests, hardening audit) were completed in nine commits:

- S1 `af57930` — remove `Action::Create` soft/restore vocabulary
- S2 `efb0ad` — remove grant-permission-to-user
- S3 `a0c51fe` — rename `Restore` → `Undelete::Soft` (Article/Version/Comment)
- S4 `c9cb178` — remove `Version::Delete::Transfer`
- S5 `7641965` — User Create/Delete::Soft/Undelete::Soft + anonymous principal
  + explicit authorization for self-service deregistration
- S6 `3e3af54` — split `Role::Manage` into 6 fine-grained permissions
- S7 `e04d9a1` — resource-type normalization + drop admin-console
  (User::Update/Delete::Hard, Role CRUD on concrete resources)
- S8 `c30da03` — reject reactivation + hide soft-deleted accounts
- S9 `c146a89` — E1 no-bypass audit (email change + session name read now
  explicitly authorized) and E2 final gate

Final gate green: back 513 tests, front 69 tests, `trunk build` succeeds,
fmt/clippy zero warnings across all three crates. Full record:
`document/exec/003_permission_overhaul.md`.