# Handoff

## Task VIII: frontend unification

**Owner**: Wk4fR7
**Exec doc**: `document/exec/F9nt_frontend-unify.md`
**Status**: DONE — all slices committed; final gate green: fmt/clippy/check clean, 82/82 tests, trunk build OK.

### Slices

- 0. Docs: exec doc `F9nt_frontend-unify.md` + this handoff. DONE (`6d80156`).
- 1. G — dead code: deleted `validate_tags` (page/validation.rs) + its unit test (80→79); deleted `RootGate` (page/session_gate.rs) + dropped unused `Outlet` import. DONE (`ba62d56`).
- 2. B1 — URL-sync helper: added `sync_url_on_change` to page/draft.rs; re-expressed `persist_draft` through it; adopted in page/article/search.rs, page/article/version/comment.rs, page/article/delete.rs. DONE (`1a4fe91`).
- 3. B2 — LevelPagination move: page/article/version/comment/pagination.rs → page/pagination.rs; adopted in page/article/version/index.rs; updated comment imports. DONE (`6724973`).
- 4. D — DeleteMode picker: new page/delete_mode.rs (`mode_to_str`, `mode_from_str(value, allowed)`, `DeleteModePicker`, `ALL_MODES`/`SOFT_AND_HARD`); adopted in article/delete.rs, version/delete.rs, comment/delete.rs; +2 tests (79→81). Skipped deregister.rs + comment/state.rs (documented, approved). DONE (`dbd2bf7`).
- 5. F — RANGE_SPECS: single `RangeSpec { range: SearchRange, label }` array in page/article/search.rs; wire via `SearchRange::as_str`; labels stay front-local; +1 test (81→82). DONE (`d98563d`).
- 6. Final gate + handoff/report. DONE.

### Decisions / notes

- A (use_remote): already done by concurrent agent — zero `use_remote` in tree; request layer unified (`request/http.rs` get_json/post_json/post_form). No code.
- E (request-layer): `validate_id` used at all 35 path-id request sites (verified). `read_tags(Option<u64>,Option<u64>)` vs `read_roles/read_users(u64,u64)` and `delete_tag → RequestResult<()>` kept as-is — unifying would change the wire (approved).
- Q1 (comment.rs:207 sync token check): KEEP + documented — async gate would change form-visible timing (approved).
- Q2 (router `/*comment_path` catch-all): KEEP + documented — restructuring changes unknown-sub-path rendering (approved).
- Q3 (deregister.rs DeleteMode radios): SKIP + documented — custom labels + hardcoded `prop:checked=true` (approved).
- G: `infrastructure/pow.rs` + `js.rs` are passthrough but USED — kept. `request/wrappers.rs` no longer exists.
- Test trajectory: 80 → 79 → 79 → 79 → 81 → 82 (net +2). Matched exactly.
- `sync_url_on_change` preserves reactive tracking: each adopter's closure reads the same signals the old Effect read, unconditionally, before returning `None`/`Some`; URL building is byte-identical (verified by diff).
- `DeleteModePicker` renders one `<div><label>` per allowed mode with `mode_to_str` text; radio `name` passed per page (delete_mode / version_delete_mode / comment_delete_mode).

### Commits

- `6d80156` docs(refactor): plan Task VIII frontend unification (exec + handoff)
- `ba62d56` refactor(front): delete dead validate_tags and RootGate
- `1a4fe91` refactor(front): extract sync_url_on_change helper
- `6724973` refactor(front): move LevelPagination to shared pagination module
- `dbd2bf7` refactor(front): unify delete mode parsing and picker
- `d98563d` refactor(front): single RANGE_SPECS array for search ranges

### Open questions

- None.

————————————————————————————————————————————————