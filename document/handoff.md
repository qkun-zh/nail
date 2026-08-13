# Handoff — nail → nail_new migration

Status of the migration plan, current state, and the remaining steps. A
fresh agent can pick up from here. The project owner adjudicates design
decisions; the migrating agent implements per this document and the
referenced artifacts. All rules live in `nail_new/README.md` (the
constitution); this document records state and process only.

## Current state (facts, verified)

- `nail` (legacy) is frozen at commit `ca59215`; `document/reference/` is the
  comment-stripped snapshot — **untrusted legacy code** (32 adjudicated
  defects, `document/adjudication.md`); regenerable via
  `git -C nail archive HEAD` + `comment-stripper-rs --strip-docs`.
- `nail_new` repo (`main`): skeleton per README §3; `document/reconstruction/`
  (reconstruct@2.17.0, fully enriched, `--check` PASSES); PRD
  `features/02-code/PRD.md` (zero unresolved callouts); `architecture/`
  INTERFACES.md (31 routes, verified), DATA-MODEL.md (37 entities, 24 enums),
  ARCHITECTURE.md (old→new mapping).
- Tooling: node v22; `/home/qkun/reconstruct-tool` (v2.17.0);
  `comment-stripper-rs` in `~/.cargo/bin`; wasm32-unknown-unknown target.

### Done

- ✅ **Phase 2 — common crate** (2026-08-13, commits `1c8d543`..`22a4f3c`):
  9 modules (`text, name, tag, response, hash, time, pow, request, search`),
  TDD red→green, **104 tests green** at `test/unit/common/<module>/tests.rs`
  via `#[path]`. Owner refinements in `document/adjudication.md`: typed
  lowercase `DeleteMode`, unified `DeleteBody`, generic `err()`, panic-free
  `token() -> anyhow::Result`, both-or-neither check, no tracing, RFC3339
  offset formatter (#19), `SearchHit` single-sourced (#10).
- ✅ **Phase 3 slice 1 — authentication/session** (2026-08-13, commit
  `6c063b0`): `GET /challenge/read`, `POST /email/read?intent=...`,
  `POST /user/create`, `GET /session/read`, `POST /session/delete` across the
  four layers; **back 59 tests green**. Back module tree = ADR-0001; `intent`
  contract = ADR-0002 (#26 closed); glossary = `document/context.md`.
  `change_email`/`deregister` branches stub 400 until slice 2.
  - §8 gate: equal-or-better on all five axes — `TokenCache<E>` replaces the
    legacy six-file token duplication, `EmailSender` seam, explicit `intent`,
    corrected error semantics (AC4/AC5). Probe finding: moka's eviction
    listener is housekeeper-driven (eventually consistent), same as legacy.

### Owner decisions (2026-08-13)

- **#26 closed**: `intent` is a query parameter, not a body field.
- **#5 confirmed**: read-open semantics — any authenticated principal may
  read article/version/comment; writes stay owner/role/admin-gated.
- Common API contracts at Phase 3 call sites: `now_ms() -> Result<u64,
  SystemTimeError>` (propagate with `?`), `uuidv7_timestamp_ms` returns
  `None` for non-v7 ids, `format_rfc3339_with_offset(utc_ms, offset_seconds)`
  accepts whole-minute offsets only (extremes ±23:59; `Z` for UTC).

## Handover (2026-08-13) — current agent

Personnel change after slice 1: the previous agent's work is committed
(`6c063b0`, `52464e2`); a new agent takes over from **slice 2 (user domain)**.

- (a) Add `#![allow(dead_code)]` at the `nail_back`/`nail_front` crate roots —
  the confirmed §5.4 mechanism, **NOT yet applied**; deleted together with the
  dead code in the Phase 5 cleanup.
- (b) ✅ Owner's `README.md` changes are committed; working tree is clean.
- (c) ✅ #5 owner-confirmed — proceed through slice 6 without stopping.

## Rules (non-negotiable; operational only)

Read `nail_new/README.md` in full first — it is the constitution and the
single source of rules; nothing is repeated here. Not in the README, still
binding:

- One task = one commit (`git add .` + commit + push) with an English message
  reflecting the actual change; archive before and after each task.
- Kill leftover backend/proxy processes after e2e (they lock the agdb file).
- Keep `document/adjudication.md` current as decisions refine; record new
  contracts as ADRs under `document/`.

## Reference materials

- `nail_new/README.md` — the constitution (read first).
- `document/adjudication.md` — the 32 verdicts to implement.
- `document/context.md` — glossary (domain-modeling output).
- `document/adr/` — 0001 (back module tree), 0002 (`intent` contract).
- `document/reconstruction/features/02-code/PRD.md` — the domain spec (FRs
  tagged [confirmed]/[inferred], acceptance criteria, test plan).
- `document/reconstruction/architecture/INTERFACES.md` — route contracts.
- `document/reconstruction/architecture/DATA-MODEL.md` — entities, enums.
- `document/reconstruction/architecture/ARCHITECTURE.md` — layer mapping.
- `document/reference/code/` — the legacy code (untrusted; probe to verify).
- `nail` (frozen) — git archaeology if a detail is missing.

## What the target is (and is not)

- The target is **PRD requirements + adjudication verdicts + the README
  architecture rules** — the *corrected* behavior, not a copy of the legacy.
- The reconstruction PRDs are an inventory (including the bugs), not a spec;
  when the legacy contradicts a verdict, the verdict wins.
- Anything new found during migration: probe it, then report to the owner —
  never silently preserve a bug or a compromise design.

## Remaining steps

### Phase 3 — backend migration, one domain per slice (in progress)

Per domain: read the PRD slice → write failing tests first (red) → implement
(green) → commit. Order:

1. ✅ Authentication/session — done (see Current state).
2. User domain — #15 (symmetric email_hash defaults, no swallowed errors),
   #27 (idempotent deregister confirm).
3. Article + version — #6, #16, #17, #18, #20, #21, #23.
4. Comment domain — create/reply/list/update/delete with the `DeleteBody`
   mode contract (#2).
5. Download/PDF — #1 (mint → `.../content/read?token={token}`, single-use
   60s, user-bound), #28 (hash-based filenames only), #29 (mint-JSON
   contract documented).
6. Role/authorization — #5 (visibility deleted; policy 2 rewritten to the
   owner-confirmed read-open semantics), #7 (member_count), #8 (REQUIRED_ROLES
   protected), #9 (duplicate role → 400).
7. Config/email/infrastructure — #13 (dead config fields), #19 (timezone from
   toml via `/config/read`), #22 (multipart read-then-validate), #26 (email
   service with explicit intent), #32 (e2e flag; test tree in Phase 5).

Layering per ARCHITECTURE.md: interface = HTTP + envelope + session-token +
PoW placement; logic = business rules (near-pure); repository = agdb + moka +
seekstorm; infrastructure = axum bootstrap, toml conf, SMTP client, PDF store,
tracing, Cedar engine.

### Phase 4 — frontend migration (after the backend API is stable)

- Layering per README §4.2 (router → page → request → infrastructure);
  Leptos CSR, no CSS (README §10); runtime config from `/config/read` with
  compile-time fallback.
- Items: #14 (all UI strings English), #24 (page size from config), #25
  (has_next = page < total_pages), #3 (no per-comment pre-check), #12 (drop
  `/private/email/check` link), #4 (delete `pow-worker.js`).
- Gate: `cargo check --target wasm32-unknown-unknown` on `nail_front`.

### Phase 5 — tests + end-to-end + cleanup

- Rebuild the test tree per README §12: unit families at the four backend
  seams + common.
- e2e strategy (#32): decide with the owner (the old design: real process
  stack back + pingap + chromium with an in-process SMTP sink).
- Final batch dead-code cleanup (README §5.4): remove every dead/unused item
  and the `#![allow(dead_code)]` crate-root attributes in one dedicated pass,
  then re-run the full `cargo test` and `cargo check` with zero warnings.
- Gate: full `cargo test` suites; e2e serial (`--test-threads=1`) if ports
  are shared.

## Skills — mandatory usage

Skills are MANDATORY, not optional: before every covered task, invoke the
matching skill via the `skill` tool and follow it — skipping one is a process
violation. The README is the constitution and outranks any skill. Do not
grill routine work.

| Skill | MUST invoke before | Notes for this repo |
| --- | --- | --- |
| setup-matt-pocock-skills | the first session, once | already done |
| tdd | every implementation slice | agree seams with the owner first; one failing test → minimal implementation; tests through public interfaces; the PRDs carry `--tdd` |
| codebase-design | every module-tree / boundary design | README §4 fixes direction only; this decides depth/shape/seams |
| diagnosing-bugs | behavior contradicts expectations | reproduce → isolate → root cause → fix; report findings to the owner |
| domain-modeling | start of Phase 3 + any new term | glossary already at `document/context.md` |
| grilling | an open/risky owner decision | one question at a time, recommend an answer each round |
| grill-with-docs | an open decision that lands as ADRs + glossary | use when the outcome must be documented |
| handoff | end of every slice, phase, and session | update `document/handoff.md`; keep adjudication current |
| to-spec | when the owner wants a formal spec | optional — PRDs + adjudication already act as specs |
| improve-codebase-architecture | periodic architecture health check | not needed during the build |
| create-skill | only if the owner asks | — |
