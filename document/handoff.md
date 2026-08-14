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
- **Phase 3 backend: DONE (slices 1-7).** back **245 tests green** (common
  104), `cargo check` zero warnings, working tree clean at `d7378e9`. No git
  remote. History + per-slice §8.3 gates: `document/progress-log.md`.
  Personnel: agent G completed Phase 3 (slice 7 + #33); a new agent takes
  over at Phase 4.
- Backend layering per ADR-0001; `intent` per ADR-0002; glossary
  `document/context.md`; Cedar engine landed (slice 6); `/config/read` returns
  the typed `common::response::RuntimeLimits` (slice 7).
- `_`-prefixed archive files removed (`aa39cb6`); pre-rewrite code in git
  (`8de3490^`). File tools work on all paths (verified). The
  `thermo-nuclear-code-quality-review` skill is unavailable in the registry —
  use the manual §8.3 gate and record it (512-line bar, README §5.3).

## Pending — current agent

- **Phase 4 — frontend migration** (next phase; the backend API is stable).
  Layering per README §4.2 (router → page → request → infrastructure); Leptos
  CSR, no CSS (README §10); runtime config from `/config/read` (reuse
  `RuntimeLimits` as the limits signal) with compile-time fallback. Items: #14
  (English UI), #24 (page size from config), #25 (has_next = page <
  total_pages), #3 (no per-comment pre-check), #12 (drop `/private/email/check`
  link), #4 (delete `pow-worker.js`). Gate: `cargo check --target
  wasm32-unknown-unknown` on `nail_front`.
- Phase 5 cleanup candidate (recorded, not urgent): sweep the slices 1-6
  `serde_json::Value`/`json!` responses to typed DTOs per the owner decision
  below.

## Owner decisions (details: adjudication.md + git log)

- #5 read-open: any authenticated principal may read; writes stay
  owner/role/admin-gated. #26 `intent` is a query parameter (ADR-0002).
- **CRUD-only vocabulary** (README §5.2): `create/read/update/delete` only;
  collection reads are `read` (never `list`); wire flow terms never appear as
  backend identifiers; interface strictest, logic top-level same verbs.
  Sanctioned exceptions: `create_reply`; `mint`/`consume` for token resources.
- **#33**: legacy policy 1's owner bypass was judged wrong (excludes
  Version::Update/Delete, Comment::Update, contradicting FR-20/21) — amended
  in `efe8cfe`: policy 1 now includes the three actions; `Comment.owner` =
  comment author and `Version.owner` = article owner confirmed in
  `repository/authorization.rs` (no assembly fix needed); 5 denial tests
  flipped to owner-allow; member seed grants NOT widened.
- **Responses are fixed data structures, not `json!` (owner, 2026-08-14)**: a
  typed-DTO sweep of the slices 1-6 responses is a Phase 5 cleanup candidate.
- **Pagination config scope (owner, 2026-08-14)**: config holds only
  frontend-facing `search_page_size` + `max_search_pages`; the backend clamp
  caps `max_search_page_size` (200) and `max_page` (10000) stay hardcoded
  constants, neither config nor served.
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

### Phase 3 — backend migration (DONE, 2026-08-14)

Slices 1-7 complete, back 245 tests green. Details per slice:
`document/progress-log.md`.

### Phase 4 — frontend migration (next)

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
