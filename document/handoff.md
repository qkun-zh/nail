# Handoff

## Task organization rules (mandatory for every handoff write/update)

1. Every task must be decomposed into a three-level hierarchy, ordered by size:
   **task → stage → slice** (task is the largest unit, stage is intermediate,
   slice is the smallest unit).
   - task is numbered with Roman numerals (e.g. `I.`, `II.`)
   - stage is numbered with capital letters (e.g. `A.`, `B.`)
   - slice is numbered with Arabic numerals (e.g. `1.`, `2.`)
2. A task, once fully complete, must be promptly removed from the handoff to
   prevent entropy explosion — keep only incomplete and in-progress entries.
3. Every slice must record its status, any information requiring the user's
   confirmation, and the user's decisions/choices.
4. Each task must have a clear boundary in the handoff (partitioned by task,
   ownership labeled) to prevent confusion and interference.
5. Do not modify, delete, or interfere with tasks not owned by you; changing
   another's task requires explicit permission.
6. The entire document must be written in English.
7. Each agent's workspace must be separated by a divider of exactly 64
   em-dashes (`—`).
8. Each task must open with a task header in exactly this form, and its
   `Owner` must be a 6-character random code (A-Z, a-z, 0-9; no name/alias):
   ```markdown
   ## Task {roman}: {short title}

   **Owner**: {6-char code}
   **Exec doc**: `document/exec/{NNN}_slug.md`
   **Status**: {one-line progress summary}
   ```

----------------------------------------------------------------

## Task I: Search Author Link + User Public Page

**Owner**: opencode
**Exec doc**: `document/exec/004_search_author_link_and_user_page.md`
**Status**: All slices done, final gate passed

### Stage A: Backend — Search Index author_id

| Slice | Status | Description |
|---|---|---|
| 1 | ✅ done | common: `SearchArticleItem` + `SearchCommentItem` add `author_id: String` |
| 2 | ✅ done | back: search index add `FIELD_AUTHOR_ID`, bump schema version "2"→"3", store/read in `document.rs` |
| 3 | ✅ done | back: `logic/search.rs` pass `author_id` through `ArticleBuilder` and `comment_to_response` |

### Stage B: Frontend — Author Links + User Page

| Slice | Status | Description |
|---|---|---|
| 4 | ✅ done | front: search result author names → clickable `<A href="/public/user/{uid}">` |
| 5 | ✅ done | front: new `/public/user/{uid}` public page + route (reuse admin detail logic) |
| 6 | ✅ done | front: remove login-based button hiding (all buttons visible, backend 403) |

### Gate

| Check | Status |
|---|---|
| back tests (513) | ✅ pass |
| front tests (69) | ✅ pass |
| trunk build | ✅ pass |
| fmt + clippy | ✅ pass |
