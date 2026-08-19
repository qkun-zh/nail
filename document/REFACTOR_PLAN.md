# REFACTOR PLAN — de-spaghetti pass (orchestration memory)

**Author**: orchestrator agent. **Purpose**: single source of truth for the whole
refactoring effort. Read this FIRST on every resume/dispatch. If context was
compressed, this file restores all decisions and state.

Repo: `/home/qkun/nail`. Constitution: `README.md`, `AGENTS.md`,
`document/workflow.md`. Workflow is MANDATORY for every code change
(baseline → pin → plan → exec doc → evidence → gate adoption → slice loop →
handoff → final gate). Test commands use nightly + cranelift + line-tables
(see `document/run.md`).

---

## 1. Approved scope (user-approved)

### Public HTTP route changes (approved A1–A4) — singular child resources
| # | old | new |
|---|---|---|
| A1 | `/version/{id}/comments/create` | `/version/{id}/comment/create` |
| A2 | `/version/{id}/comments/read` | `/version/{id}/comment/read` |
| A3 | `/comments/{id}/replies/create` | `/comment/{id}/reply/create` |
| A4 | `/comment/{id}/replies/read` | `/comment/{id}/reply/read` |

### Public JSON wire changes (approved B1–B6) — unify list pages to
`ListPage<T> { items: Vec<T>, has_next: bool, total: u64 }`
| # | endpoint | field rename |
|---|---|---|
| B1 | tag list | `tag_list` → `items` |
| B2 | role list | `role_list` → `items` |
| B3 | user list | `user_list` → `items` |
| B4 | version list | `version_list` → `items` (add `total`) |
| B5 | article search | `article_list` → `items` (drop echoed `page`; frontend tracks page locally) |
| B6 | comment list | `comments` → `items` (add `total`) |

Placeholder key renames (`{role_id}`→`{id}`) are INTERNAL only (URL unchanged).
`{version_id}`/`{tag_id}` stay (two ids in one path).

### Decisions (user, locked)
- D1: Proceed item-by-item; each task decomposes into stages/slices.
- D2: Dirty tree: commit directly (DONE, see §5).
- D3: Dead code: delete immediately (no `#[allow(dead_code)]` residue).
- D4: Repository response-assembly → logic: **ORCHESTRATOR DECISION = DEFER**
  (high-risk/invasive; do the low-risk refactors first; revisit only if budget
  allows). Recorded so future-me doesn't re-litigate.
- D5: email fixes: **NOT in scope** (double-lock, cooldown-before-success,
  per-send transport rebuild). Do not touch `logic/email.rs` /
  `infrastructure/email*.rs` behavior.
- D6: `has_consistent_email_pow_pair` (common/src/request.rs:44): DELETE.
- D7: `delete.rs` latest-version string-max bug: FIX (write failing repro first).

---

## 2. Resource / test policy (user-mandated)

- Machine is resource-limited: a single full `cargo test` can OOM.
- Tests MUST be run in parts, serially, but ALL of them must eventually pass.
- Split: per-crate (`common`, then `back`, then `front`); within a big crate,
  further split per module if needed. Sub-agents run STRICTLY SERIALLY (one at a
  time). Never start a build on a busy machine (check `uptime`/`ps` first;
  back off if loaded) per workflow §8.
- Final gate = every crate: `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test`; frontend also `trunk build`. All green before a task is
  complete.

---

## 3. Task decomposition (task → stage → slice)

Owner codes: 6-char random, assigned per task. Exec docs: `document/exec/<4-char>_<slug>.md`.
Handoff files: `document/handoff/<4-char>_<slug>.md`.

- **Task I — Clean tree.** DONE. Commits: `1426da4` (fix tag route),
  `6cd6d61` (user list feature), `881b7b4` (docs).
- **Task II — common validators unification.** common/src/{validate,name,tag,text}.rs + tests.
  Preserve per-policy behavior (name/tag: `-`/`_`; text: printable+newline). One
  `CharPolicy` + one `ValidationError`. No wire change.
- **Task III — common named-struct dedup + dead code.** (REORDERED LATER — touches
  frontend `request/*` which a concurrent agent is actively refactoring; dispatch
  after concurrent frontend work stabilizes. Re-verify tree first.)
  Merge TagRef/TagNameView/RoleNameView → `NamedRef`; collapse view==listitem
  duplicates (keep RoleListItem which genuinely differs); delete
  `has_consistent_email_pow_pair`; SearchRange single Display/FromStr+label
  (keep same serde strings → no wire change). Update all back/front consumers.
- **Task IV — ListPage<T> unification (WIRE B1–B6).** (REORDERED LATER — touches
  frontend list pages; same concurrent-conflict reason.)
  Atomic across common+back+front+tests. common defines `ListPage<T>{items,has_next,total}`;
  back produces it; front reads `items`; http tests updated. Drop `page` echo.
- **Task V — repository graph abstraction.** graph.rs add
  `outgoing_edges/incoming_edges/edge_count`; kill ~30 dup query blocks; kill
  `_sync/_in_txn` duplication (macro/trait); single `highest_version_number`
  + fix delete.rs string-max bug (failing repro first); unify edge insert via
  `insert_edge`; delete dead `read_tag_articles`.
- **Task VI — logic unification.** pagination via `logic/pagination.rs`
  everywhere; error mapping (`From<RepoError> for LogicError`, `database_error`
  maps not-found→404 not 500); shared soft-delete/undelete/transfer ops; token
  lifecycle helper; content-hash dedup helper; remove empty `logic/*/` dirs +
  dead `read_tag_detail`; move download URL build + version fs out of logic.
- **Task VII — interface/infrastructure.** `AppPaged<T>` extractor (kill 6
  pagination clamps + 5 param structs); multipart extractor (move
  read_text_field/stream_pdf_field/map_multipart_error out of article handler);
  unify error construction (kill side-effecting `From<LogicError>`, standardize
  3 styles); route singularization A1–A4 (atomic with front request/comment.rs
  + http tests); optional session header extractor; body-limit accessor +
  monster const rename + `{role_id}`→`{id}`.
- **Task VIII — frontend unification.** `use_remote<T>` hook (kill ~12
  async-load dupes); single URL-sync helper (kill 4 impls); session status
  signal (kill inline auth + fix comment router catch-all); DeleteMode picker +
  shared mode string helpers; standardize request-layer body/return; search.rs
  decompose (`Vec<RangeSpec>`); delete dead `validate_tags`/thin wrappers.
- **Task IX — (DEFERRED, see D4)** repository response-assembly → logic.
- **Task X — final cleanup + final gate.** remove remaining dead fields/allows
  (OffsetTime.offset, principal allows); full per-crate test+clippy+fmt+trunk.

---

## 4. Sub-agent operating rules (I am the user for them)

Each sub-agent MUST, in order:
1. Read `README.md`, `AGENTS.md`, `document/workflow.md`, this plan, and the
   relevant exec/handoff docs.
2. Follow workflow exactly: baseline green → pin → plan → write
   `document/exec/<code>_<slug>.md` → evidence (source+probe for unknowns) →
   I approve (adoption gate) → slice loop (red→green→gate: fmt/clippy/test) →
   one commit per slice on clean tree → update handoff file → report.
3. Check machine load before any build; back off if loaded. Run tests split
   per crate. Never `--release`.
4. No `unwrap`/`expect`/new panics. No comments restating code. Config in toml.
5. After finishing, leave `document/handoff/<task>_<slug>.md` current and
   report to me. I review; on pass I push, on fail I rollback (revert the
   commit) and send it back.
6. Do NOT touch tasks owned by other agents; do NOT modify files outside the
   task's declared file list without flagging in exec doc.

---

## 5. State log (append; never delete)

- 2026-08-19: Dirty tree committed (Task I done): `1426da4`,`6cd6d61`,`881b7b4`.
- 2026-08-19: Master plan created (this file). Scope A1–A4, B1–B6 approved.
- 2026-08-19: Task II DONE (reviewed, committed): `454e4a2` (shared validate module), `f030c6f` (name/tag), `cb8243c` (text), `71fc173` (exec+handoff docs). Behavior/messages preserved; 116/116 common tests green; fmt/clippy clean; no-ripple check clean in back/front.
- 2026-08-19: CONCURRENT AGENT active on shared local `main`: committed `5ceb421` (feat(front): validate path ids) + more (branch ahead of origin/main); staged uncommitted frontend changes incl. new `code/front/src/request/validate.rs`, `test/unit/front/request/validate/tests.rs`, edits to `request/{article,comment,download,role,tag,user,version}.rs`. PUSH HELD: pushing local main would publish un-audited concurrent work. Coordination needed before any push.
- 2026-08-19: ORCHESTRATOR DECISION (recorded): (1) REORDER — dispatch backend-only tasks (V, VI, VII-backend) before frontend-touching tasks (III, IV, VIII, VII-routes) to avoid clashing with the concurrent agent's frontend `request/*` refactor; re-verify the tree at every dispatch. (2) HOLD ALL PUSHES until the concurrent agent's work on local `main` is stable/audited or the user directs a coordinated push; committing locally is safe (commits are independent).
- 2026-08-19: Task II committed on `main` (local). Next: Task V (repository graph abstraction).
- 2026-08-19: Task V DONE (reviewed, approved): commits `29f56aa`,`d203237`,`15a4681`,`6b49a93`,`8f5a9d4`,`9b6586e`,`b6fbd94`,`c0aa715`. -1090/+626 lines. GraphQuery/GraphWrite traits unify executor; edge helpers kill ~30 dup query blocks; `highest_version_number` (semver) fixes delete.rs string-max bug with genuine red-first repro (left "1.0.0" right "9.9.9"); dead `read_tag_detail`/`read_tag_articles`(prod) removed. Back gate green: fmt clean, clippy 0 warnings, 543/543 tests. FOLLOW-UPS recorded: (a) migrate `repository/user.rs` off the 3 remaining `_sync` helpers (`find_by_index_sync`,`read_rows_sync`,`resolve_node_id_sync`) → natural task; (b) single-variant `_sync` domain fns in delete.rs/comment.rs could be renamed later.
- 2026-08-19: Task VIa DONE (reviewed, approved): commits `401366f`,`07f2d4f`,`58956df`,`4aed1ef`,`85076c0`,`a633402`,`de1e26b`. paginate/page_offset unify 7 sites; hash_token/hash_canonical_token unify 12 token sites (message consolidation approved, unreachable paths); reject_duplicate_content_hash shared; empty logic/*/ dirs + root .gitkeep removed; user.rs migrated off _sync, 3 base _sync helpers deleted from graph.rs. Probe 003 found REAL latent page-0 has_next disagreement (div_ceil vs search's len>offset+limit); unified on div_ceil — page 0 unreachable via interface (clamped >=1), documented. Back gate: fmt clean, clippy 0, 556/556 tests (per-module; single binary OOMs/SIGKILLs). NOTE for Task X: final gate MUST run tests per-module or per-crate, not one big binary.
- 2026-08-19: Dispatch Task VII-backend next (AppPaged extractor, multipart extractor, error-construction unify, body-limit accessor + const/placeholder renames, optional session header). Pure backend, no frontend. Then VIb (error-mapping unify, behavior change) and VII-routes (A1-A4, touches frontend) and frontend tasks (III/IV/VIII) after concurrent frontend work stabilizes.
- 2026-08-19: Task VII-backend DONE (reviewed, approved): commits `2afd3a6`,`d774f3c`,`20fe7e5`,`b651aac`,`6408ebf`,`3ecdb96`,`7357345`. AppPaged (non-generic) kills 6 clamp blocks + 5 param structs; interface/multipart.rs with collect_fields unifies field-scan loops; ApiError::from_logic/with_status replace side-effecting From<LogicError> (52 sites, tracing preserved); ServerConfig::max_request_body_bytes() accessor + const rename + {role_id}→{id} (URL values unchanged, probe-verified); read_session_token shared. 576/576 tests green (baseline 556+20), fmt clean, clippy 0. DEVIATION: agent used `sed -i` (forbidden) once, self-corrected with Edit tool; final state verified correct by orchestrator review. NOTE: pre-existing dev server PID 198643 + pingap proxy run in env — do NOT kill.
- 2026-08-19: Next: Task VII-routes (A1–A4, user-approved; touches front request/comment.rs — check concurrent-agent activity first), then VIb (error-mapping unify, BEHAVIOR-PRESERVING decision: keep statuses, add From impls, centralize; the 404-vs-500 improvement is a follow-up needing explicit user approval), then Task X final gate. Frontend tasks III/IV/VIII after concurrent frontend work settles.
- 2026-08-19: Task VII-routes DONE (reviewed, approved): commits `9548d0f` (route change), `b350302` (docs). A1–A4 exactly as approved: router constants singularized, 30 http-test literals, cedar.rs const assertion, front request/comment.rs 4 segment swaps. 576/576 back tests, fmt clean, clippy 0, front `cargo +nightly check` clean. Remaining `data.comments` JSON field untouched (belongs to Task IV wire change).
- 2026-08-19: Dispatch Task VIb next (backend-only error-mapping unification, behavior-preserving).
- 2026-08-19: Task VIb DONE (reviewed, approved): commits `985f89b`,`657509d`,`72956ab`,`b263dc3`. 9 `From<RepoError> for LogicError` impls in error.rs (exhaustive, messages byte-identical), MAX_COMMENT_TREE_DEPTH moved, 7 named mappers deleted, all DbError sites → `?` (red proven: 67 E0277 / 7 E0425). Per-site overrides kept where genuinely different (comment reply/delete-transfer, admin-name UserMissing, email full match, read_comment_children special case, 15 custom-message DbError wraps). 583/583 green (baseline 574 + 9 From tests), fmt clean, clippy 0, no deviations.
- 2026-08-19: Dispatch Task IV next (ListPage<T> wire unification B1–B6, user-approved). Frontend currently stable (tree clean). Remaining after IV: Task VIII (frontend unify — large), Task III (common NamedRef dedup), Task X (final cleanup + final gate). If orchestrator context runs low: REFACTOR_PLAN.md + handoff files carry full state; a fresh session resumes from here.
- 2026-08-19: Task IV DONE (reviewed, approved): commits `fa24f69` (common ListPage<T> + back producers + count helpers + tests), `24d3262` (front consumers + search page-echo removal), `9198a41` (docs). 48 files. ListPage<T>{items,has_next,total} in common/src/response.rs; six endpoints unified; no stale wire field names; search tracks page locally via requested_page; total semantics: len-before-slice (search in-memory), version/comment via new same-filter count helpers. Gates: common 117/117, back 583/583 (per-module; single-process full suite OOM-killed — always per-module), front 80/80 + trunk build, fmt/clippy clean.
- 2026-08-19: Dispatch Task III next (common NamedRef merge + TagView/TagListItem collapse + SearchRange single source + delete has_consistent_email_pow_pair). NO wire changes in this task (beyond B1-B6 already done): ArticleListItem collapse is DEFERRED (would add fields to article list wire — needs approval).
- 2026-08-19: REMAINING after Task III: Task VIII (frontend unify: use_remote, URL-sync helper, session gate, DeleteMode picker, request-layer conventions, search.rs decompose, dead validate_tags/wrappers) and Task X (final cleanup: OffsetTime.offset dead field, principal allows, + FINAL GATE: all crates full test per-module + fmt + clippy + trunk). Deferred by D4: repository response-assembly → logic.
- 2026-08-19: Task III DONE (reviewed, approved): commits `2728f91` (NamedRef merge), `3b53129` (TagView→TagListItem), `fd6efda` (SearchRange as_str/FromStr single wire source, 12 strings 1:1, manual serde), `956d8b4` (dead: has_consistent_email_pow_pair + NameSetRequest + DeregisterUserRequest + DeregisterUserConfirmRequest, zero refs), `5d0aa29` (docs). 25 files. Gates: common 117/117, back 583/583, front 80/80 + trunk build, clippy 0. NO wire changes. Deviation: sed -i used once in slice 1 (2nd occurrence in project), self-corrected + verified. Handoff flags RANGE_KEYS dup in front search.rs → route via SearchRange::as_str in Task VIII.
- 2026-08-19: Dispatch Task VIII next (frontend unification — LAST big refactor). After VIII: Task X (final cleanup + FINAL GATE) closes the project. D4 (repository response-assembly → logic) remains deferred.
- 2026-08-19: If orchestrator context runs low during VIII/X: REFACTOR_PLAN.md + exec/handoff docs carry full state; fresh session resumes per plan + handoff readme.
