# Handoff — nail → nail_new migration

Pickup doc for the current migrating agent. Rules live in `nail_new/README.md`
(the constitution — read it in full first). Full history, per-slice §8.3 gates,
and probe findings: `document/progress-log.md`. Adjudication verdicts:
`document/adjudication.md` (32 + #33).

## Current state (facts)

- `nail` (legacy) frozen at `ca59215`; `document/reference/` = comment-stripped
  snapshot, **untrusted** (33 adjudicated defects). Reconstruction docs pass
  `--check`; PRD `features/02-code/PRD.md` = the domain spec; INTERFACES /
  DATA-MODEL / ARCHITECTURE under `document/reconstruction/architecture/`.
- **Phase 3 backend: slices 1-6 done.** back **240 tests green** (common 104),
  `cargo check` zero warnings, working tree clean at `29955cb`. No git remote.
- Backend layering per ADR-0001; `intent` per ADR-0002; glossary
  `document/context.md`; Cedar engine landed in slice 6
  (`infrastructure/cedar/{schema,policy}.cedar` + `repository/authorization.rs`).
- `_`-prefixed archive files removed (`aa39cb6`); pre-rewrite code in git
  (`8de3490^`). File tools work on all paths (verified). The
  `thermo-nuclear-code-quality-review` skill is unavailable in the registry —
  use the manual §8.3 gate and record it (512-line bar, README §5.3).

## Pending — current agent

1. **#33 owner-bypass patch** (one commit): amend policy 1 in
   `code/back/src/infrastructure/cedar/policy.cedar` to add
   `Version::Update`/`Version::Delete`/`Comment::Update` to the owner bypass
   (`resource.owner == principal`); verify `Comment.owner` = comment author,
   `Version.owner` = article owner in `repository/authorization.rs` (fix if
   not); flip slice 6's 5 denial tests to owner-allow; add a
   comment-author ≠ article-owner test. Do NOT widen the member role's seed
   grants (non-owner-scoped). Details: adjudication #33 + Owner decisions.
2. **Slice 7 — config/email/infrastructure** (last backend slice, TDD): #13
   (dead config `db_namespace`/`db_database`/`max_id_filter_count`), #19
   (timezone from toml via `/config/read`), #22 (multipart read-then-validate,
   K), #26 (email explicit intent — verify convergence with ADR-0002), #32
   (e2e flag + test tree → Phase 5). FR-1..8 leftovers: config validation
   matrix (fail fast → `startup-errors.log`, exit 1), `/config/read` route
   (README §11), per-minute log rotation + retention prune, graceful shutdown
   (search-index close already wired). §8.2 pre-study: legacy `other/conf.rs`,
   `other/log.rs`, `api/meta.rs`.

## Owner decisions (details: adjudication.md + git log)

- #5 read-open: any authenticated principal may read; writes stay
  owner/role/admin-gated. #26 `intent` is a query parameter (ADR-0002).
- **CRUD-only vocabulary** (README §5.2): `create/read/update/delete` only;
  collection reads are `read` (never `list`); wire flow terms never appear as
  backend identifiers; interface strictest, logic top-level same verbs.
  Sanctioned exceptions: `create_reply`; `mint`/`consume` for token resources.
- **#33**: legacy policy 1's owner bypass was judged wrong (excludes
  Version::Update/Delete, Comment::Update, contradicting FR-20/21) — amend
  policy 1; do not widen member seed grants.
- Common contracts: `now_ms() -> Result<u64, SystemTimeError>`; `uuidv7_*`
  return `None` for non-v7 ids; `format_rfc3339_with_offset` whole-minute
  offsets only.

## Rules (non-negotiable; README is the constitution)

- One task = one commit (`git add .` + commit; push is moot — no remote) with
  an English message; keep the tree clean.
- README §13: terminal `cd` must be `qkun`-prefixed; never use the diagnostics
  tool; grep only in the terminal.
- Never propagate unverified claims into this file (tooling/anomaly claims need
  the exact path and error text).

## Reference materials

- `README.md` — constitution. `document/adjudication.md` — 33 verdicts.
  `document/context.md` — glossary. `document/adr/` — 0001, 0002.
  `document/progress-log.md` — history + §8.3 gates.
- `document/reconstruction/features/02-code/PRD.md` — the spec (FR-1..66).
- `document/reconstruction/architecture/` — INTERFACES (31 routes),
  DATA-MODEL, ARCHITECTURE.
- `document/reference/code/` — untrusted legacy; probe to verify. `nail`
  (frozen) — git archaeology.

## Remaining steps

### Phase 4 — frontend migration (after the backend API is stable)

Layering per README §4.2 (router → page → request → infrastructure); Leptos
CSR, no CSS (README §10); runtime config from `/config/read` with
compile-time fallback. Items: #14 (English UI), #24 (page size from config),
#25 (has_next = page < total_pages), #3 (no per-comment pre-check), #12 (drop
`/private/email/check` link), #4 (delete `pow-worker.js`). Gate:
`cargo check --target wasm32-unknown-unknown` on `nail_front`.

### Phase 5 — tests + e2e + cleanup

Rebuild the test tree per README §12 (unit families at the four seams +
common); e2e strategy (#32) with the owner; final batch dead-code cleanup
(README §5.4) in one pass, then zero-warning gate.

## Skills — mandatory usage

Invoke the matching skill via the `skill` tool before each covered task; the
README outranks any skill. Do not grill routine work.

| Skill | MUST invoke before | Notes |
| --- | --- | --- |
| tdd | every implementation slice | red → green; PRDs carry `--tdd` |
| codebase-design | module-tree / boundary design | README §4 fixes direction only |
| diagnosing-bugs | behavior contradicts expectations | reproduce → isolate → root cause → fix |
| handoff | end of every slice/phase/session | update this file; keep adjudication current |
| domain-modeling | any new domain term | glossary at `document/context.md` |
| grilling / grill-with-docs | an open/risky owner decision | one question at a time, recommend an answer |
| thermo-nuclear-code-quality-review | §8.3 gate per slice | unavailable in registry — manual §8.3 gate + record; 512-line bar |
| setup-matt-pocock-skills | first session, once | already done |
