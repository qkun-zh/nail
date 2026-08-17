# Handoff

## Current state

search.rs split complete. All tests green.

## Done

- Phase 1-10: Split search.rs (772→359 lines)
  - search/schema.rs (186 lines): field constants + index config
  - search/query.rs (29 lines): range-to-field mapping
  - search/db.rs (222 lines): DB enrichment helpers
  - search/document.rs (325 lines): document building (pre-existing)
- common: 109 tests, back: 454 tests — all pass
- fmt clean, no new clippy warnings

## Decisions

- DB helpers (enrich_comment_headers, article_ids_of_user, etc.) moved to db.rs — they are SearchIndex::read/sync helpers, not core search logic
- query.rs only has range-field mapping — small but cohesive
- schema.rs holds all SeekStorm field definitions and index metadata

## Remaining risks

- None. Pure refactoring, no behavior change.

## Next

- Pending: Permission system overhaul (see below)

---

# Permission System Overhaul

## Decisions

1. `Restore` → `Undelete::Soft` (consistent naming)
2. Delete `User::Delete::Transfer`, keep `Version::Delete::Transfer`
3. Split `Role::Manage` into 6 permissions: Create/Read/Update/Delete/Grant/Revoke
4. Virtual unified: remove `Virtual::"admin-console"`, use action set matching
5. User supports soft delete

## Permission Count: 30 (was 27)

### Article (7)
- Create, Read, Update, Delete::Hard, Delete::Transfer, Delete::Soft, Undelete::Soft

### Version (7)
- Create, Read, Update, Delete::Hard, Delete::Transfer, Delete::Soft, Undelete::Soft

### Comment (7)
- Create, Read, Update, Delete::Hard, Delete::Transfer, Delete::Soft, Undelete::Soft

### User (4)
- Read, Update, Delete::Hard, Delete::Soft

### Role (6)
- Create, Read, Update, Delete, Grant, Revoke

---

## TODO

### Phase 1: Cedar Schema (`schema.cedar`)
- [ ] Rename `Article::Restore` → `Article::Undelete::Soft`
- [ ] Rename `Version::Restore` → `Version::Undelete::Soft`
- [ ] Rename `Comment::Restore` → `Comment::Undelete::Soft`
- [ ] Delete `User::Delete::Transfer` action
- [ ] Add `User::Delete::Soft` action
- [ ] Delete `Role::Manage` action
- [ ] Add `Role::Create`, `Role::Read`, `Role::Update`, `Role::Delete`, `Role::Grant` actions
- [ ] Change all `resource: [Virtual]` to unified `Virtual`

### Phase 2: Cedar Policy (`policy.cedar`)
- [ ] Policy 1: Add `User::Delete::Soft` to owner bypass, remove `User::Delete::Transfer`
- [ ] Policy 4: Change to action set matching instead of resource name
- [ ] Policy 5: Update recycler restrictions (remove Transfer for User)
- [ ] Verify admin role protection still works

### Phase 3: Build Script (`build.rs`)
- [ ] Update test_only list: remove `User::Delete::Transfer`, add `User::Delete::Soft`

### Phase 4: Repository Layer
- [ ] `repository/role.rs`: Constants auto-generated, verify new permission names
- [ ] `repository/delete.rs`: Add `soft_delete_user` function
- [ ] `repository/authorization.rs`: Update `Resource::Virtual` handling

### Phase 5: Logic Layer
- [ ] `logic/article.rs`: Rename `restore_article` → `undelete_soft_article`
- [ ] `logic/version.rs`: Rename `restore_version` → `undelete_soft_version`
- [ ] `logic/comment.rs`: Rename `restore_comment` → `undelete_soft_comment`
- [ ] `logic/user.rs`: Remove transfer mode, add soft delete mode
- [ ] `logic/role.rs`: Update to use 6 fine-grained permissions

### Phase 6: Interface Layer
- [ ] `interface/router.rs`: Update route names (restore → undelete-soft)
- [ ] `interface/article.rs`: Update handler names
- [ ] `interface/version.rs`: Update handler names
- [ ] `interface/comment.rs`: Update handler names
- [ ] `interface/role.rs`: Update permission checks

### Phase 7: Operations (`logic/operations.rs`)
- [ ] Update ROUTE_*_RESTORE → ROUTE_*_UNDELETE_SOFT
- [ ] Update ROLE_* permission mappings

### Phase 8: Tests
- [ ] Update all test files using old permission names
- [ ] Update all test files using old route names
- [ ] Add tests for User::Delete::Soft
- [ ] Verify all tests pass

### Phase 9: Verification
- [ ] `cargo fmt`
- [ ] `cargo clippy`
- [ ] `cargo test` (all 563+ tests pass)
- [ ] `trunk build` (frontend)
