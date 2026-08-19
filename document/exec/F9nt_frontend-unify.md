# Exec — Task VIII: frontend unification

**Task**: VIII (REFACTOR_PLAN.md §Task VIII). **Owner**: Wk4fR7.
**Scope root**: `code/front/src/**`, `test/unit/front/**`. No `code/back/**`, no
`code/common/**`.

## 1. Requirement

Behavior-preserving frontend unification (the_shit.md §四/§五 → REFACTOR_PLAN
§Task VIII). Targets A–G; a concurrent agent rebuilt the frontend request
layer since the review, so several targets are already done — verify, then
only add where missing. Hard constraints: no wire change (request layer, HTTP
payloads, query params), no behavior change, no visual change. No
`unwrap`/`expect`/new panics. No comments restating code. English only.

Acceptance: front `cargo +nightly check` clean; front unit tests green
(baseline **80/80**); `trunk build` OK; `cargo +nightly fmt --check` clean;
clippy `-D warnings` zero; one commit per slice on a clean tree.

## 2. Inventory (evidence, gathered 2026-08-20)

### A. use_remote — ZERO copies remain (task item already done)

- `grep use_remote` over `code/front/src` → 0 matches. `code/front/src/api/`
  does not exist.
- Request layer rebuilt by concurrent agent: `request.rs` +
  `request/{article,auth,comment,download,envelope,error,http,pow,role,
  session,tag,url,user,validate,version}.rs`. Shared primitives:
  `http::{get_json,post_json,post_form}` (http.rs:75,93,116),
  `envelope::{parse_envelope,unwrap_envelope}`, `error::RequestResult`.
- **Decision**: A is DONE. No code change; document as evidence.

### B. URL-sync sites (review: 4 impls)

1. `page/draft.rs:6-49` — `build_draft_query` (drops empty, encodes) +
   `draft_url` + `persist_draft` (Effect, skip-first, navigate
   replace:true/resolve:false). THE shared abstraction; used by 7 sites
   (create, authenticate, update, email/update, version/create, deregister,
   name/update).
2. `page/article/search.rs:126-156` sync_url closure + Effect at 295-307.
   Reads q_filter/ranges/from_time/to_time/current_page; pre-encodes; always
   pushes `ranges=` and `page=`; navigates `{SEARCH_PATHNAME}?{query}`.
3. `page/article/version/comment.rs:61-107` sync_url closure + Effect at
   109-119. Reads body/reply_body/update_body/comment_path/page; pathname
   depends on mode; EARLY RETURN for Delete/Undelete/Invalid modes (no nav);
   drops-empty via draft_url; page always pushed.
4. `page/article/delete.rs:46-69` sync_url closure + Effect at 63-69. Reads
   mode + article_id; single `mode=` param; early return if no article_id.
5. `page/article/version/index.rs:71-89` — hand-rolled PrevNext + on_go
   (navigate `{base}?page={target}`) — duplicates `LevelPagination`
   (comment/pagination.rs:9-37), which does exactly that.

Design: one helper `sync_url_on_change(navigate, build)` in draft.rs — the
shared part is the Effect skeleton + skip-first + NavigateOptions; each site
keeps its own URL-building closure (byte-identical URLs), returning
`Option<String>` (None = skip navigation). `persist_draft` is re-expressed
via the helper internally (its 7 call sites unchanged). `LevelPagination`
moves to `page/pagination.rs` (from comment/pagination.rs) and is adopted by
version/index.rs; comment/index.rs + comment/detail.rs update the import.

### C. Session gate — EXISTS (task item mostly done)

- `page/session_gate.rs`: `SessionStatus`, `provide_session_state`,
  `use_session_status`, `mark_session_invalid`, `authenticated_user_id`,
  `refresh_session`, `who_are_you`, `RootGate`.
- Adopted at: main.rs:14-15, logout.rs:4,26, author_gate.rs:46,
  comment/detail.rs:6,39, email/update.rs:7,80,113, comment/index.rs:6,37,
  deregister.rs:9,63,82, comment.rs:25,236,243,255, name/update.rs:8,18,48,68,
  authenticate.rs:7,73.
- Remaining hand-rolled: comment.rs:207 `!read_session_token().
  unwrap_or_default().is_empty()` — a SYNCHRONOUS token-presence check for
  form visibility. The gate is async (Checking → Authenticated/Anonymous);
  adopting it would change form-visible timing and stale-token UX. Task says
  "kill inline auth", constraint says no behavior change → **Q1**.
- Router catch-all: router.rs:88-91 `/*comment_path` → CommentSection,
  reverse-parsed by comment/url.rs. Fixing = routing behavior change for
  unknown sub-paths → **Q2**.
- `RootGate` (session_gate.rs:80) — zero references → delete (slice 1).

### D. DeleteMode sites (4)

1. `page/article/delete.rs:10-25` — `mode_to_str` + `mode_from_str`
   (transfer/hard/soft) + radio picker (109-125) + sync_url uses mode_to_str.
2. `page/article/version/delete.rs:8-14` — `mode_from_str` (soft/hard ONLY,
   no transfer) + radio picker (69-89). No sync_url.
3. `page/article/version/comment/delete.rs:9-32` — comment_delete_view radio
   picker (transfer/soft/hard); no string helpers; no sync_url.
4. `page/user/deregister.rs:100-101` — transfer/soft ONLY, DIFFERENT labels
   ("Transfer (content moves to platform)"), transfer radio has hardcoded
   `prop:checked=true` (not signal-bound).
5. `comment/state.rs:355-359` — DeleteMode → success-message strings
   (comment-specific wording).

Genuine differences: version/delete.rs restricts to soft/hard; deregister has
custom labels + hardcoded checked. Design: new `page/delete_mode.rs` with
`mode_to_str`, `mode_from_str(value, allowed: &[DeleteMode])`, and
`DeleteModePicker(mode, name, options: &[(DeleteMode, &'static str)])`
(div>label>input rows, identical DOM). Adopt in delete.rs / version/delete.rs
(allowed=[Soft,Hard]) / comment/delete.rs. Skip deregister.rs + state.rs
(documented).

### E. Request-layer conventions — mostly done

- `request/validate.rs` `validate_id` used by EVERY path-id request fn
  (article 4, version 7, comment 8, role 3, tag 6, user 5, download 2 = 35
  sites, verified).
- Remaining inconsistencies: `read_tags(Option<u64>,Option<u64>)` vs
  `read_roles/read_users(u64,u64)` (tag/list.rs:15, tag_picker.rs:12 pass
  None,None → page/limit omitted from query = current wire); `delete_tag`
  returns `RequestResult<()>` (backend returns `{"id":...}`, front ignores
  it). Unifying either = wire/type change → **keep + document** (constraint:
  no wire change).

### F. search.rs — 334 lines; submodules already exist

- Submodules: search/{comments,form,results,versions}.rs (concurrent agent).
  Remaining: RANGE_KEYS (19-32) + RANGE_LABELS (33-46) parallel arrays +
  `checked_range_subset` (49-57) + coordinator (~254 lines).
- RANGE_KEYS EXACTLY matches `SearchRange::as_str` (common/search.rs:24-39),
  12 strings 1:1, SAME ORDER (title, summary, author_name, comment, note,
  tag, version_number, article_id, version_id, comment_id, author_id, role).
- RANGE_LABELS DIFFER from `SearchRange::label` ("author name" vs "author",
  "version note" vs "note", "version number" vs "version") → labels must stay
  front-local (routing through `label` changes displayed text = visual
  change). RANGE_KEYS used at search.rs:103 + 49-57; RANGE_LABELS at
  form.rs:4,34.
- Design: one front-local `const RANGE_SPECS: [RangeSpec; 12]` where
  `RangeSpec { range: SearchRange, label: &'static str }` (order = current
  RANGE_KEYS order); wire keys via `range.as_str()`, labels stay as today.
  Kills the parallel-array index coupling (review §四.3) AND the RANGE_KEYS
  wire-string duplication (Task III handoff flag).

### G. Dead code

- `page/validation.rs:41-48` `validate_tags` (`#[allow(dead_code)]`) — zero
  production refs; only its own test (test/unit/front/page/validation/
  tests.rs:49-60 `tags_mirror_the_backend_parser`). DELETE fn + test
  (80 → 79 tests).
- `page/session_gate.rs:80` `RootGate` — zero refs (component: no dead_code
  warning). DELETE; drop `Outlet` from its `use leptos_router::components`
  import (`A` stays — used by who_are_you).
- `infrastructure/pow.rs` + `js.rs` — pure passthrough BUT USED (pow →
  request/pow.rs:9; js → create.rs:88, version/create.rs:72). NOT dead →
  keep, document.
- `request/wrappers.rs` — does not exist anymore. N/A.

## 3. Slice breakdown

| # | Stage | Red | Green | Exit test |
|---|---|---|---|---|
| 0 | docs | — | exec + handoff files | — |
| 1 | G | grep: validate_tags/RootGate zero refs (evidence) | delete validate_tags + its test; delete RootGate + Outlet import | fmt, clippy, test 79, check |
| 2 | B1 | N/A (pure refactor); existing 79 tests pin behavior | draft.rs `sync_url_on_change` + persist_draft re-expressed; adopt search.rs/comment.rs/delete.rs | fmt, clippy, test 79, check |
| 3 | B2 | N/A (pure move) | LevelPagination → page/pagination.rs; adopt version/index.rs; comment imports updated | fmt, clippy, test 79, check |
| 4 | D | new mode_to_str/mode_from_str tests (compile-fail: missing module) | page/delete_mode.rs + 3 adopters | fmt, clippy, test 81, check |
| 5 | F | new RANGE_SPECS test (round-trip via SearchRange::from_str) | RANGE_SPECS single source; search.rs + form.rs rewired | fmt, clippy, test 82, check |
| 6 | final | — | full gate incl. trunk build | all green |

Test trajectory: 80 → 79 → 79 → 79 → 81 → 82.

## 4. Open unknowns

- U1 (in-tree proof): `persist_draft` already reads `fields()` BEFORE the
  `previous.is_none()` check (draft.rs:32-33) — the same pattern the new
  helper uses; Effect tracking is established by reads inside the closure.
  No probe needed.
- U2 (source): version/index.rs PrevNext and LevelPagination both render
  `PrevNext` with default class "pagination" (pagination.rs:78) and
  `has_prev = current > 1` — DOM identical. Verified by reading both.
- U3 (behavior): comment.rs sync_url must keep its unconditional 5-signal
  read (body/reply/update/comment_path/page) inside the new closure so
  tracking is preserved in delete modes — closure mirrors today's tuple.

## 5. Verification plan

- Behavior: identical by construction — each slice's URL-building / DOM
  markup is byte-for-byte the same; only the Effect skeleton and
  NavigateOptions move into the helper. Radio DOM identical
  (div>label>input, same name per page, same order).
- No wire change: no request-layer signature/body edits (E documented only).
- Per-slice: fmt, clippy `-D warnings`, `cargo +nightly test` (counts above),
  `cargo +nightly check`. Final gate adds `trunk build`.
- Test/run commands MUST use the run.md flags
  (`CARGO_PROFILE_DEV_DEBUG=line-tables-only RUSTFLAGS=-Zcodegen-backend=
  cranelift`).

## 6. Risks

- Concurrent agents may touch `code/front/src/**` — re-read files before each
  slice; stage only own paths; never `git add -A`/`git add .`.
- Behavior drift in sync_url refactor: mitigated by keeping each site's URL
  builder closure byte-identical; the helper only owns Effect+options.
- Machine load: check `uptime` before every build; run tests single-crate
  (front only), never `--release`.
- Two untracked `code/back/rustc-ice-*.txt` crash dumps exist (not mine) —
  never stage/discard.

## 7. Constraints

- No wire/behavior/visual change. No `unwrap`/`expect`/new panics. No
  comments restating code. English only. Never hand-edit Cargo.lock. One
  commit per slice, clean tree. Stage only own paths. Back/common untouched.

## 8. Questions

- Q1: comment.rs:207 inline auth (sync token-presence check). Adopting the
  async session gate changes form-visible timing + stale-token UX (behavior
  change). Plan: KEEP + document. OK?
- Q2: router.rs:88 `/*comment_path` catch-all. Restructuring into explicit
  routes changes unknown-sub-path rendering (behavior). Plan: KEEP +
  document (God component already decomposed by concurrent agent). OK?
- Q3: deregister.rs DeleteMode radios (custom labels + hardcoded
  `prop:checked=true`). Plan: SKIP + document. OK?

## Change log

- 2026-08-20: exec doc written; baseline verified green (check clean, 80/80
  tests, trunk build OK, fmt clean, clippy 0). Awaiting adoption gate.