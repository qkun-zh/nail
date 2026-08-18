# Tag CRUD — Independent Tag Management

**Owner**: pending
**Status**: Planning — exec doc written

## Requirement

Tags become a first-class entity with independent CRUD, like Role is to User.
Tag-Article association (apply/unapply) happens through article create/update.
Only admin can create tags; authors select from existing tags.

## Acceptance Criteria

1. Tag CRUD endpoints: create, read (list + single), update (rename), delete
2. Tag delete: cascade remove edges, delete tag node, do not affect articles
3. Article create/update: validate tag names exist, reject unknown tags
4. Article create/update forms: multi-select from existing tag list (replaces textarea)
5. Frontend tag management pages: list, create, detail (with article list), update, delete
6. Only admin can create tags (PERMISSION_TAG_CREATE)
7. All existing tests pass; new tests for tag operations

## Scope

### In-scope

- Backend: Cedar schema, permissions, repository, logic, interface, router for Tag
- Frontend: tag pages, tag request functions, article form tag picker
- Article create/update tag validation (reject unknown tags)

### Out-of-scope

- Tag search/filter beyond list pagination
- Tag usage statistics
- Bulk tag operations
- Tag synonyms/merging

## Design Decisions

### Permission model

- `Tag::Create` — admin only (applies to Virtual resource)
- `Tag::Read` — any authenticated user (applies to Tag, Virtual)
- `Tag::Update` — admin only (applies to Tag)
- `Tag::Delete` — admin only (applies to Tag)

No `Tag::Apply` / `Tag::Unapply` — association is through article create/update,
covered by `Article::Update` permission.

### Tag delete strategy

Cascade: remove all article-tag edges for the tag, then delete the tag node.
Articles are not affected. Existing article-tag edges are simply broken.

### Route design

```
Backend:
  POST   /tag/create
  GET    /tag/read
  GET    /tag/{id}/read
  POST   /tag/{id}/update
  POST   /tag/{id}/delete

Frontend:
  /tag                → list
  /tag/create         → create form (admin only)
  /tag/:id            → detail (name + article list)
  /tag/:id/update     → rename form
  /tag/:id/delete     → delete confirmation
```

### Article forms

Replace textarea with multi-select checkbox list. Fetch `GET /tag/read` on form
load, render checkboxes, send selected tag names with article create/update.

### Files touched

**Backend (8 files):**

| File | Change |
|------|--------|
| `code/back/src/infrastructure/cedar/schema.cedar` | Add Tag entity + 4 actions |
| `code/common/src/response/tag.rs` | NEW: TagView, TagListItem, TagListPage, TagNameView |
| `code/common/src/response.rs` | Add `pub mod tag` |
| `code/common/src/request.rs` | Add CreateTagRequest, TagUpdateRequest |
| `code/back/src/repository/tag.rs` | Add read_tags, read_tag, update_tag, delete_tag |
| `code/back/src/logic/tag.rs` | NEW: validate, CRUD logic, authorization |
| `code/back/src/interface/tag.rs` | NEW: 5 HTTP handlers |
| `code/back/src/interface/router.rs` | Add 5 tag routes + constants |
| `code/back/src/logic/operations.rs` | Add tag route actions |
| `code/back/src/logic/article.rs` | Validate tag names exist in create/update |

**Frontend (10 files):**

| File | Change |
|------|--------|
| `code/front/src/request/tag.rs` | NEW: tag API functions |
| `code/front/src/page/tag.rs` | NEW: module declarations |
| `code/front/src/page/tag/list.rs` | NEW: tag list page |
| `code/front/src/page/tag/create.rs` | NEW: tag create page |
| `code/front/src/page/tag/detail.rs` | NEW: tag detail page |
| `code/front/src/page/tag/update.rs` | NEW: tag update page |
| `code/front/src/page/tag/delete.rs` | NEW: tag delete page |
| `code/front/src/page.rs` | Add `pub mod tag` |
| `code/front/src/router.rs` | Add tag routes |
| `code/front/src/page/article/create.rs` | Replace textarea with tag multi-select |
| `code/front/src/page/article/update.rs` | Replace textarea with tag multi-select |

## Slice Breakdown

### Slice 1: Backend tag CRUD

1. Cedar schema: add `Tag` entity + 4 actions (Create, Read, Update, Delete)
2. `common/src/response/tag.rs`: TagView, TagListItem, TagListPage, TagNameView
3. `common/src/request.rs`: CreateTagRequest, TagUpdateRequest
4. `repository/tag.rs`: read_tags, read_tag, update_tag, delete_tag
5. `logic/tag.rs`: validate_tag_name, create_role pattern → tag CRUD
6. `interface/tag.rs`: 5 HTTP handlers
7. `interface/router.rs`: 5 routes
8. `logic/operations.rs`: route action mappings

**Exit test**: `cargo build` passes, existing tests pass.

### Slice 2: Article tag validation

1. `logic/article.rs` create: validate all tag names exist in DB
2. `logic/article.rs` update: validate all tag names exist in DB
3. Reject with clear error if any tag name not found

**Exit test**: article create with non-existent tag name returns error.

### Slice 3: Frontend tag pages + request functions

1. `request/tag.rs`: create_tag, read_tags, read_tag, update_tag, delete_tag
2. `page/tag/`: list, create, detail, update, delete pages
3. `page.rs`: add `pub mod tag`
4. `router.rs`: add tag routes

**Exit test**: `trunk build` passes.

### Slice 4: Article form tag picker

1. `page/article/create.rs`: fetch tags, render multi-select checkboxes
2. `page/article/update.rs`: fetch tags, render multi-select with pre-checked current tags

**Exit test**: `trunk build` passes.

### Slice 5: Tests + final gate

1. Tag CRUD tests (create, read, update, delete, list)
2. Article create with non-existent tag → error
3. Article create with valid tags → edges created
4. Tag delete → edges removed, articles unaffected
5. Full gate: `cargo test` + `cargo clippy` + `cargo fmt` + `trunk build`

**Exit test**: all tests green, zero warnings, clean fmt.

## Open Unknowns

- Tag create permission: confirmed admin-only (user decision)
- Tag delete cascade: confirmed remove edges only (user decision)
- Article form UX: confirmed multi-select from list (user decision)

## Verification Plan

| Dimension | Method |
|-----------|--------|
| Correctness | Unit tests for tag CRUD, integration test for article+tag flow |
| Behavior change | Article create rejects unknown tags; tag list page exists |
| Time complexity | Tag list is O(n) with pagination; no regression |
| Space complexity | Tag nodes + edges; same as current |
| Performance | No change to hot paths |

## Risks

- Article create/update now requires extra DB lookup to validate tags → minimal overhead
- Tag delete cascade may be slow with many edges → acceptable for expected scale
- Multi-select UI may need debounce for large tag lists → pagination handles this

## Constraints

- No `unwrap`/`expect` (README robustness)
- No comments restating code (README comments)
- Files ≤512 lines, functions ≤256 lines
- English only
- nightly + Cranelift for dev builds

## Questions

None — all design decisions confirmed by user.
