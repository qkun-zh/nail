# Q4z9 Permissions Coverage — All 39 Cedar Actions in Frontend

**Owner**: Q4z9
**Exec doc**: `document/exec/Q4z9_permissions-coverage.md`
**Status**: Slice 2 done (f3dac8f); Slice 3 (frontend) next

## Task Q4z9: expose all 39 Cedar authorization actions in the frontend

### Stage A: Backend

#### Slice 1: Role `{role_id}` routing + 39 Cedar actions + tag apply/unapply

**Status**: DONE — commit `345f69e`

- Role routes switched from `{name}` to `{role_id}`; Tag::Apply/Unapply added
  (39 actions); tag apply/unapply routes + logic handlers; ROUTE_ACTIONS covers
  all three User::Delete::* actions.
- Evidence: 525/525 back tests; clippy zero warnings; fmt clean.
- Tests: cedar count 37→39; logic+http role rewritten id-based (47 tests);
  `logic/tag_apply.rs` (9) + `http/tag_apply.rs` (3).
- Decisions: apply/unapply admin-only by default (follow Role::Grant/Revoke);
  member gets Forbidden.

#### Slice 2: Soft-delete visibility gating in logic reads

**Status**: DONE — commit `f3dac8f`

- `require_visible_if_soft_deleted` helper in `logic/authorize.rs`; repository
  single reads unfiltered (article/version/comment); gates in logic read_article/
  read_version/read_comment/read_user/read_comments (version-level) and
  download mint (Version::Undelete::Soft). Lists keep repository filtering.
- Evidence: 531/531 back tests; clippy zero warnings; fmt clean.
- Tests: `logic/soft_delete_visibility.rs` (6 new); updated repository/delete.rs
  (5), logic/delete_verify.rs (3), logic/version.rs (2), logic/user.rs (1).
- User decisions: none required — follows design decision 4 in exec doc.

### Stage B: Frontend

#### Slice 3: Role module + undelete/update/apply pages

**Status**: PENDING

- `request/role.rs`, `page/role/` (list/create/detail/update/delete);
  tag apply/unapply pages; version/comment update/delete/undelete pages;
  user undelete page; router wiring.
- Exit test: `trunk build` passes; each new route navigable.

#### Slice 4: Tests + final gate

**Status**: PENDING

- Full gate: back/common/front tests + clippy + fmt + trunk build; handoff;
  delete exec doc.

## Questions for the user

- None outstanding; per directive, proceeding without asking unless business
  design is uncertain.