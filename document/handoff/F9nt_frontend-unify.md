# Handoff

## Task VIII: frontend unification

**Owner**: Wk4fR7
**Exec doc**: `document/exec/F9nt_frontend-unify.md`
**Status**: IN PROGRESS — slice 0 (docs) done; baseline 80/80 green, check/fmt/clippy clean, trunk OK.

### Slices

- 0. Docs: exec doc `F9nt_frontend-unify.md` + this handoff. DONE.
- 1. G — dead code: delete `validate_tags` (page/validation.rs:41) + its unit test (80→79); delete `RootGate` (page/session_gate.rs:80) + drop unused `Outlet` import. PENDING.
- 2. B1 — URL-sync helper: add `sync_url_on_change` to page/draft.rs; re-express `persist_draft` through it; adopt in page/article/search.rs, page/article/version/comment.rs, page/article/delete.rs. PENDING.
- 3. B2 — LevelPagination move: page/article/version/comment/pagination.rs → page/pagination.rs; adopt in page/article/version/index.rs; update comment imports. PENDING.
- 4. D — DeleteMode picker: new page/delete_mode.rs (`mode_to_str`, `mode_from_str(value, allowed)`, `DeleteModePicker`); adopt in article/delete.rs, version/delete.rs, comment/delete.rs; +2 tests (79→81). Skip deregister.rs + comment/state.rs (documented, approved).
- 5. F — RANGE_SPECS: single `RangeSpec { range: SearchRange, label }` array in page/article/search.rs; wire via `SearchRange::as_str`; labels stay front-local; +1 test (81→82).
- 6. Final gate + handoff/report.

### Decisions / notes

- A (use_remote): already done by concurrent agent — zero `use_remote` in tree; request layer unified (`request/http.rs` get_json/post_json/post_form). No code.
- E (request-layer): `validate_id` used at all 35 path-id request sites (verified). `read_tags(Option<u64>,Option<u64>)` vs `read_roles/read_users(u64,u64)` and `delete_tag → RequestResult<()>` kept as-is — unifying would change the wire (approved).
- Q1 (comment.rs:207 sync token check): KEEP + document — async gate would change form-visible timing (approved).
- Q2 (router `/*comment_path` catch-all): KEEP + document — restructuring changes unknown-sub-path rendering (approved).
- Q3 (deregister.rs DeleteMode radios): SKIP + document — custom labels + hardcoded `prop:checked=true` (approved).
- G: `infrastructure/pow.rs` + `js.rs` are passthrough but USED — kept. `request/wrappers.rs` no longer exists.
- Test trajectory: 80 → 79 → 79 → 79 → 81 → 82 (net +2).

### Commits

- (slice 0) docs commit pending.

### Open questions

- None.

————————————————————————————————————————————————