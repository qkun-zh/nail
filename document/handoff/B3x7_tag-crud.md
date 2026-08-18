# Handoff

## Task I: Tag CRUD — Independent Tag Management

**Owner**: B3x7
**Exec doc**: `document/exec/B3x7_tag-crud.md`
**Status**: Slice 3 complete — 3/5 slices done

### Stages

A. ✅ Backend tag CRUD (Cedar schema, permissions, repository, logic, interface, router)
B. ✅ Article tag validation (reject unknown tags in create/update)
C. ✅ Frontend tag pages + request functions
D. ⬜ Article form tag picker (multi-select from existing tags)
E. ⬜ Tests + final gate

### Decisions made

- Tag CRUD follows Role pattern (create, read list, read single, update, delete)
- Only admin can create tags (PERMISSION_TAG_CREATE)
- Tag delete: cascade remove edges, delete tag node, articles unaffected
- Article create/update: validate tag names exist, reject unknown
- Article forms: multi-select checkboxes from tag list (replaces textarea)
- Routes use tag ID (not name) as path parameter
- No separate apply/unapply endpoints — association through article create/update
- No TAG_APPLY/TAG_UNAPPLY permissions — ARTICLE_UPDATE covers it
- TagNameView now includes `id` field for frontend navigation

### Code changes

**Backend:**
- `code/back/src/infrastructure/cedar/schema.cedar`: Tag entity + 4 actions
- `code/common/src/response/tag.rs`: TagNameView now has `id` field
- `code/common/src/request.rs`: CreateTagRequest, TagUpdateRequest
- `code/back/src/repository/tag.rs`: Full CRUD + article query functions
- `code/back/src/repository/authorization.rs`: Tag variant in Resource enum
- `code/back/src/logic/tag.rs`: Tag CRUD logic + authorization
- `code/back/src/interface/tag.rs`: 5 HTTP handlers
- `code/back/src/interface/router.rs`: 5 tag routes
- `code/back/src/logic/operations.rs`: Tag route action mappings
- `code/back/src/logic/article.rs`: Tag validation in create/update

**Frontend:**
- `code/front/src/request/tag.rs`: Tag CRUD request functions
- `code/front/src/page/tag/`: list, detail, create, update, delete pages
- `code/front/src/router.rs`: Tag routes added

**Tests:**
- `test/unit/back/infrastructure/cedar.rs`: Updated action count to 37
- `test/unit/back/infrastructure/cedar_probe.rs`: Updated action count to 37
- `test/unit/back/repository/search.rs`: Updated schema version to 5
- `test/unit/back/context.rs`: Added create_tag, seed_tags helpers
- All test files with article creation: Added tag seeding
- 513 tests pass

### Remaining

- Slice 4: Article form tag picker (multi-select checkboxes)
- Slice 5: Tests + final gate
