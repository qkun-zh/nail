# Q5nd — Task III: common named-struct dedup + dead code (NO wire changes)

## 1. Requirement

Behavior-preserving type-level dedup in `common`, zero JSON wire delta. Four
changes, all with type names/files only:

- **Stage A**: merge byte-identical `TagRef` (`common/src/tag.rs`),
  `TagNameView` (`common/src/response/tag.rs`), `RoleNameView`
  (`common/src/response/role.rs`) into one `NamedRef { id, name }`; update all
  consumers in common/back/front. Wire stays `{"id","name"}`.
- **Stage B**: `TagView` == `TagListItem` (both `{id,name,article_count}`).
  Keep ONE (decision: keep `TagListItem`), update the single `TagView`
  consumer. Wire stays `{id,name,article_count}`.
- **Stage C**: `SearchRange` wire string becomes single source:
  `as_str()`/`from_str()` producing the EXACT 12 strings the serde rename
  produces today; `label()` stays the single humanization point; manual serde
  impls route through `as_str`/`from_str` (removing the duplicated rename
  strings); `logic/search.rs` re-parse routes through `FromStr`. No wire string
  changes.
- **Stage D**: delete dead `has_consistent_email_pow_pair`
  (`common/src/request.rs:44-49`) plus 3 verified-dead request structs
  `NameSetRequest`, `DeregisterUserRequest`, `DeregisterUserConfirmRequest`
  (zero production consumers).

Acceptance: common 117 tests green (117 baseline − 1 deleted dead-fn test + 1
new search test), back 583 green per-module, front `cargo +nightly check` +
unit tests green, fmt/clippy clean, `git diff` shows zero wire-string
literals changed on the JSON side.

## 2. Scope

In-scope: the four stages' files below; common/back/front consumers; unit
tests referencing the removed types; the request tests covering dead items.

Out-of-scope (explicit): `RoleListItem` vs `RoleView` (genuinely differ —
`member_count` vs `members`; NOT collapsed). `ArticleListItem` subset of
`ArticleView` (would add fields to article-list wire; orchestrator-deferred).
Frontend `RANGE_KEYS` copy of the search wire strings in
`code/front/src/page/article/search.rs:19-32` (duplication exists but belongs
to Task VIII's search.rs decompose; touching it here is scope creep).
Repository-internal `RoleView` (`repository/role.rs`, `repository/
authorization.rs`) is a DIFFERENT struct — untouched. Untracked
`code/back/rustc-ice-*.txt` dumps: not mine, not touched, not committed.

## 3. Design decisions

1. `NamedRef` lives at `common/src/response.rs` root (next to `ListPage`,
   `RuntimeLimits`, `EmptyView` — the shared wire-type layer), derives
   `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`, no rename attrs →
   wire `{"id","name"}` identical to all three today.
2. Stage B keeps `TagListItem` (5 consumer files) over `TagView` (1 consumer
   file) — least churn; `read_tag` (detail) returns the same-shaped type.
3. Stage C manual serde: unit-variant derive → `serialize_unit_variant` →
   `serialize_str(renamed_name)` (serde_derive-1.0.229 ser.rs:527,
   serde_json-1.0.122 ser.rs:210-215 — source evidence below). Manual impls
   `serialize_str(self.as_str())` + string visitor via `from_str` are
   byte-equivalent; serde `rename_all`/`rename` attributes are REMOVED so the
   wire strings exist in exactly one place (`as_str`). `from_str` error
   payload carries the unknown token so `logic/search.rs` keeps its exact
   message `unknown search range: {token}`.
4. `TokenRequest`/`LogoutRequest` (still live) stay; only dead-struct
   segments are stripped from `single_pow_requests_round_trip`.
5. Slice atomicity: front depends on common via path; common type removal
   breaks back/front compile until their slices land. Order: common-first
   slices (1, 2), then back/front consumers in the same slice commits —
   slices 1/2 each atomically cover common+back+front, so the tree is green
   after every commit.

## 4. Slice breakdown

- **Slice 1 — NamedRef merge (Stage A).** Files: `common/src/response.rs`
  (add), `common/src/tag.rs`, `common/src/response/tag.rs`,
  `common/src/response/role.rs`, `common/src/response/article.rs`,
  `code/back/src/repository/article.rs`, `code/back/src/repository/tag.rs`,
  `code/back/src/interface/tag.rs`, `code/back/src/interface/role.rs`,
  `code/back/src/logic/role.rs`, `code/front/src/request/tag.rs`,
  `code/front/src/request/role.rs`, `code/front/src/page/tag/{update,detail,
  delete}.rs`, `code/front/src/page/role/delete.rs`,
  `test/unit/common/tag/tests.rs` (TagRef wire test → NamedRef).
  Red: `tag/tests.rs` rewritten to `NamedRef` fails to compile before merge.
  Green: common tests, back per-module, front check.
- **Slice 2 — TagView collapse (Stage B).** Files: `common/src/response/tag.rs`
  (delete TagView), `code/back/src/logic/tag.rs`. Red: after common deletion,
  `cargo +nightly check` on back fails E0432 (inventory proves sole consumer).
  Green: back per-module + common + front check.
- **Slice 3 — SearchRange single source (Stage C).** Files:
  `common/src/search.rs`, `code/back/src/logic/search.rs`,
  `test/unit/common/search/tests.rs` (new `as_str`↔wire identity test).
  Red: new test referencing `as_str`/`from_str` fails to compile.
  Green: existing search wire tests (serialize/deserialize/reject) prove zero
  string drift; logic search tests prove re-parse path.
- **Slice 4 — dead code (Stage D).** Files: `common/src/request.rs`,
  `test/unit/common/request/tests.rs`. Red: N/A (pure deletion; D6
  pre-approved; compiler is the check — any missed reference = E0432).
  Green: common tests 117 (one dead-fn test removed, one search test added
  net-zero), back/front unchanged compile.

## 5. Open unknowns (evidence)

| Unknown | Evidence |
|---|---|
| All `TagRef`/`TagNameView`/`RoleNameView`/`TagView`/`TagListItem` consumers | grep inventory below (source) — exhaustive across code/{common,back,front}/src + test/ |
| `has_consistent_email_pow_pair` dead? | repo grep: only its own unit test (test/unit/common/request/tests.rs:197,204,211,218) — no back/front use (source) |
| `NameSetRequest`/`DeregisterUserRequest`/`DeregisterUserConfirmRequest` dead? | repo grep: zero hits in code/back/src + code/front/src; only common unit tests (source) |
| Manual serde impl wire-equivalence | serde_derive-1.0.229 ser.rs:527 (unit variant → serialize_unit_variant), serde_json-1.0.122 ser.rs:210-215 (→ serialize_str(variant)) — source; probe = existing tests `search/tests.rs` (all 12 wire strings + round-trip + rejection hard-asserted) + `response/tests.rs` SearchHit round-trip — they run green today and MUST stay green post-impl (probe) |
| Wire shapes of tag/role endpoints | http tests tag_apply.rs:33-46,96,111 (article_count on read; id/name on list) and role.rs:67-71,149-150 (`data.name`/`data.id`) — source |
| `logic/search.rs` error message stability | logic/search.rs:259 `unknown search range: {token}` — kept verbatim via from_str error payload (source) |

## 6. Verification plan

- Correctness: existing common wire tests (search 4, tag 1, response round
  trips) + back http tests (tag_apply 2, role) + logic search tests — all must
  stay green unchanged; new search as_str↔wire identity test.
- Behavior change: input/output delta vs baseline = NONE (diff review; only
  type names/files move). Deltas: −1 dead-fn test +1 new test (same wire).
- Time/space: O(1) enum as_str/from_str (match), same allocations (zero
  String alloc in as_str). Rename slices: zero runtime delta.
- Performance: N/A (no hot-path change; manual serde = same string ops).
- Wire identity: `git diff` review of every touched response-producing path +
  http tests green.

## 7. Risks

- Missed consumer → compile error; mitigated by exhaustive inventory below +
  per-crate gates (back E0432 would name the file).
- serde manual impl subtlety → existing wire tests are the tripwire (4 search
  tests + SearchHit round trip cover all 12 variants both directions +
  rejection).
- Front breaks mid-task → slices 1/2 each include their front consumers
  atomically (unlike Task IV, no cross-slice front dependency).
- Concurrent agent edits → re-read files before each slice; stage only my
  paths; per-slice commit on clean tree.
- Rollback: revert slice commits (linear).

## 8. Constraints

- No wire changes; if a consumer needed one to convert, STOP and report.
- No `unwrap`/`expect`/new panics. No comments restating code. English only.
  No hand-edited Cargo.lock. Read/Edit/Write only (no sed/awk/cat>).
- Check load before every build; per-module serial test runs.
- Only the declared file list; anything else flagged to orchestrator.

## 9. Questions

1. Stage D beyond the named method: 3 verified-dead request structs
   (`NameSetRequest`, `DeregisterUserRequest`, `DeregisterUserConfirmRequest`)
   + their test segments — delete (D3/D6 spirit) or keep? Evidence: zero
   production consumers. Plan assumes delete.
2. NamedRef placement at `response.rs` root vs a new `response/named.rs`
   module — plan assumes root (matches ListPage/EmptyView pattern).

## Consumer inventory (every type use)

### TagRef → NamedRef
- `code/common/src/tag.rs:10` def; `code/common/src/response/article.rs:3`
  (import), `:13` (field)
- `code/back/src/repository/article.rs:4,43,370,378`
- `code/back/src/repository/tag.rs:2,16,18,36`
- `test/unit/common/tag/tests.rs:4,105,114`

### TagNameView → NamedRef
- `code/back/src/interface/tag.rs:5,23,64`
- `code/front/src/request/tag.rs:4,29,35,43`
- `code/front/src/page/tag/update.rs:6,13`; `detail.rs:5,10`; `delete.rs:6,13`

### RoleNameView → NamedRef
- `code/back/src/logic/role.rs:12,110,186,216`
- `code/back/src/interface/role.rs:5,22`
- `code/front/src/request/role.rs:5,25,55`
- `code/front/src/page/role/delete.rs:4,14,26`

### TagView → TagListItem (delete TagView)
- `code/back/src/logic/tag.rs:14,64,70`

### TagListItem (kept; consumers unchanged, verified)
- `code/back/src/logic/tag.rs:14,38,47`; `code/front/src/request/tag.rs:4,13`;
  `code/front/src/page/tag/list.rs:6,10`;
  `code/front/src/page/article/tag_picker.rs:3,7`;
  `test/unit/common/response/tests.rs:104,110`

### SearchRange (Stage C)
- `common/src/search.rs` def; `common/src/response/search.rs:3,7`
- `code/back/src/logic/search.rs:7,235-269` (re-parse match → FromStr)
- `code/back/src/repository/search.rs:4,31,40`;
  `repository/search/document.rs:4,55-87`; `repository/search/query.rs:1,9-33`
- front: indirect via `SearchArticleItem`/`SearchHit` deserialization (no code
  change); `test/unit/common/search/tests.rs` (wire locks)

### NOT changed (verified distinct)
- `repository/role.rs`/`repository/authorization.rs` `RoleView` (repo-internal,
  different struct)
- `RoleListItem` (differs from RoleView), `ArticleListItem` (deferred)
- front `RANGE_KEYS`/`RANGE_LABELS` (page/article/search.rs:19-47) — Task VIII

## Change log

- 2026-08-19: created. Baseline green: common 117/117, back 583/583
  (config 11, infra 45, logic 281, repo 107, http 139; two tests match both
  `logic_` and `repository_` filters by name substring, unique total 583).
  Tree clean (2 untracked rustc-ice dumps, not mine).
- 2026-08-19: evidence complete (serde source read; dead-code greps; http
  wire asserts). Awaiting adoption gate.