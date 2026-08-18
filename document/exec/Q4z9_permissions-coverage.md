# Permissions Coverage — Expose All 39 Cedar Actions in Frontend

**Owner**: Q4z9
**Status**: Planning — exec doc written

## Requirement

Every Cedar authorization action (39 total) is reachable from the frontend UI
through a CRUD-style route, so no permission is unreachable by an operator with
the grant. Currently 37 actions exist; 22/37 are exposed, 15 are not. This task
adds the missing routes/pages AND adds the two missing actions (Tag::Apply,
Tag::Unapply), bringing the total to 39 and coverage to 39/39.

Additionally, the Role entity's path parameter switches from `{name}` to
`{role_id}` for consistency with every other entity (User, Article, Version,
Comment, Tag all use ID).

## Acceptance Criteria

1. Role routes use `{role_id}` (not `{name}`) in backend and frontend.
2. Cedar schema adds `Tag::Apply` and `Tag::Unapply`; total = 39 actions; the
   stale "27 actions" comment is corrected to 39.
3. All 39 actions map to a frontend route (see coverage table in §Design).
4. Soft-deleted resources are visible only to principals with the matching
   `*::Undelete::Soft` permission, decided entirely in the backend logic layer;
   no `is_deleted` response field; frontend only renders what the backend
   returns.
5. Delete and Undelete are symmetric: every entity with a delete route also has
   an undelete route.
6. `operations.rs` ROUTE_ACTIONS includes all three `User::Delete::*` actions on
   the `/user/{uid}/delete` mapping (currently missing Transfer and Soft).
7. All tests pass; zero clippy warnings; fmt clean; trunk build succeeds.

## Scope

### In-scope

- Backend: Cedar schema (2 new actions + comment), Role `{role_id}` routing,
  logic-layer soft-delete visibility gating, ROUTE_ACTIONS completeness,
  tag apply/unapply handlers.
- Frontend: undelete pages (article/version/comment/user), version
  update/delete pages, comment update page, tag apply/unapply pages, role
  module (list/create/detail/update/delete).
- Route actions table updated; test counts updated.

### Out-of-scope

- Search-index visibility of soft-deleted resources (remains excluded).
- New soft-delete permissions beyond the existing Undelete::Soft set.
- Removing or renaming existing actions.
- Frontend cosmetic/UX work beyond what is required to reach a route.

## Design Decisions

### 1. Visibility is a backend concern

No `is_deleted` response field. `read_*` reads the row, and if the row is
soft-deleted, requires `*::Undelete::Soft`; otherwise requires `*::Read`. No
permission → not-found. Frontend renders whatever the backend returns.

```rust
pub async fn read_article(state, actor_id, article_id) -> Result<ArticleView, LogicError> {
    let article = read_article_from_db(&state.graph, article_id).await?
        .ok_or_else(|| LogicError::not_found("article not found"))?;
    if is_soft_deleted(&article) {
        authorize_or(state, actor_id, PERMISSION_ARTICLE_UNDELETE_SOFT,
            &Resource::Article(article_id), "article not found").await?;
    } else {
        authorize_or(state, actor_id, PERMISSION_ARTICLE_READ,
            &Resource::Article(article_id), "article not found").await?;
    }
    Ok(article.into_view())
}
```

### 2. New actions follow the Role pattern

Role has `Grant`/`Revoke` for the user-role relationship. Tag gets `Apply`/
`Unapply` for the article-tag relationship. Both apply to the `Tag` resource.

```cedar
action "Tag::Apply"    appliesTo { principal: [User], resource: [Tag] };
action "Tag::Unapply"  appliesTo { principal: [User], resource: [Tag] };
```

### 3. Route coverage table (39/39)

| Action | Route | Frontend |
|--------|-------|----------|
| Article::Create | /article/create | existing |
| Article::Read | /article/{id}/read | existing |
| Article::Update | /article/{id}/update | existing |
| Article::Delete::Hard/Transfer/Soft | /article/{id}/delete | existing |
| Article::Undelete::Soft | /article/{id}/undelete-soft | **new** |
| Version::Create | /article/{id}/version/create | existing |
| Version::Read | /version/{id}/read | existing |
| Version::Update | /version/{id}/update | **new** |
| Version::Delete::Hard/Soft | /version/{id}/delete | **new** |
| Version::Undelete::Soft | /version/{id}/undelete-soft | **new** |
| Comment::Create | /comment/create | existing |
| Comment::Read | /comment/{id}/read | existing |
| Comment::Update | /comment/{id}/update | **new** |
| Comment::Delete::Hard/Transfer/Soft | /comment/{id}/delete | existing |
| Comment::Undelete::Soft | /comment/{id}/undelete-soft | **new** |
| User::Create | /user/create | existing |
| User::Read | /user/{uid}/read | existing |
| User::Update | /user/{uid}/update | existing |
| User::Delete::Hard/Transfer/Soft | /user/{uid}/delete | existing |
| User::Undelete::Soft | /user/{uid}/undelete-soft | **new** |
| Role::Create | /role/create | **new** |
| Role::Read | /role/{id}/read | **new** |
| Role::Update | /role/{id}/update | **new** |
| Role::Delete | /role/{id}/delete | **new** |
| Role::Grant | /role/{id}/update | **new** |
| Role::Revoke | /role/{id}/update | **new** |
| Tag::Create | /tag/create | existing |
| Tag::Read | /tag/{id}/read | existing |
| Tag::Update | /tag/{id}/update | existing |
| Tag::Delete | /tag/{id}/delete | existing |
| Tag::Apply | /article/{id}/tag/{tag_id}/apply | **new** |
| Tag::Unapply | /article/{id}/tag/{tag_id}/unapply | **new** |

## Slice Breakdown

### Slice 1: Backend — Role `{role_id}` + Cedar new actions + ROUTE_ACTIONS

1. `schema.cedar`: add `Tag::Apply`/`Tag::Unapply`, fix "27 actions" → "39".
2. `interface/router.rs`: `{name}` → `{role_id}` route constants; add tag
   apply/unapply routes.
3. `logic/role.rs`, `interface/role.rs`, `repository/role.rs`: accept
   `role_id` (resolve by ID).
4. `operations.rs`: add `User::Delete::Transfer`/`Soft` to USER_DELETE; map new
   tag apply/unapply + role routes.
5. `logic/tag.rs`: add `apply_tag`/`unapply_tag` handlers.

**Exit test**: `cargo test -- --test-threads=2` passes; count 39 actions.

### Slice 2: Backend — soft-delete visibility gating

1. `logic/article.rs`/`version.rs`/`comment.rs`/`user.rs`: `read_*` requires
   Undelete::Soft when the row is soft-deleted; remove repository-layer
   soft-delete filters for these reads so logic can decide.

**Exit test**: unit tests: soft-deleted row + Undelete permission → readable;
without → not-found.

**Status: DONE** — commit `f3dac8f`. `require_visible_if_soft_deleted` helper
in `logic/authorize.rs`; repository single reads unfiltered
(`read_comment_item_any_sync` for comments); `logic/download.rs` download mint
gated on Version::Undelete::Soft (downloads count as reads). Lists keep
repository-side filtering. Evidence: 531/531 back tests, clippy zero warnings,
fmt clean. Tests: `logic/soft_delete_visibility.rs` (6 new),
`logic/user.rs::read_user_hides_a_soft_deleted_account_from_members`,
updated `repository/delete.rs` (5), `logic/delete_verify.rs` (3),
`logic/version.rs` (2).

### Slice 3: Frontend — role module + undelete/update/apply pages

1. `request/role.rs`, `page/role/` (list/create/detail/update/delete).
2. `request/tag.rs`: apply/unapply; `page/article/apply_tag.rs`, `unapply_tag.rs`.
3. `request/version.rs`/`comment.rs`: update/delete/undelete; corresponding pages.
4. `page/user/undelete_soft.rs` and others.
5. `router.rs`: wire all new routes.

**Exit test**: `trunk build` passes; each new route navigable.

### Slice 4: Tests + final gate

1. Tag apply/unapply authorization tests.
2. Role `{role_id}` route tests.
3. Soft-delete visibility gating tests.
4. Update cedar action-count tests to 39.
5. Full gate: `cargo test` + `cargo clippy -D warnings` + `cargo fmt --check`
   + `trunk build`.

**Exit test**: all green, zero warnings, fmt clean.

## Open Unknowns

- Whether repository-layer soft-delete filters can be cleanly removed without
  breaking list/search (must keep lists excluding deleted) — **source**:
  `repository/{article,version,comment}.rs`, `logic/*.rs`.
- Role read-by-id: existing `read_role(name)` resolves via `resolve_node_id_sync`
  on name; need a read-by-id path — **source**: `repository/role.rs`.

## Verification Plan

| Dimension | Method |
|-----------|--------|
| Correctness | Unit tests per slice: role routes, tag apply/unapply, visibility gating |
| Behavior change | 37→39 actions; role `{name}`→`{id}`; soft-deleted reads require Undelete |
| Time complexity | No change to hot paths (visibility adds one branch) |
| Space complexity | No new allocations beyond routes/actions |
| Performance | No change to list/search query shape |

## Risks

- Removing repository soft-delete filters could leak into list/search. Mitigate:
  keep filters on list/search paths, gate only single-read paths in logic.
- Role `{name}`→`{id}` touches callers/tests. Mitigate: update all references in
  one slice; grep for `ROLE_NAME`.
- Search index already excludes soft-deleted; not in scope, so no index rebuild.

## Constraints

- No `unwrap`/`expect`/new panics (README robustness).
- No comments restating code (README comments).
- Files ≤512 lines, functions ≤256 lines.
- English only.
- nightly + Cranelift for dev builds (see `document/run.md`).
- Never hand-edit `Cargo.lock`; no secrets.

## Questions

None — all design decisions confirmed by user in conversation.
