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
