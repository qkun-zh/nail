# Handoff — nail → nail_new migration

Status of the migration plan, current state, and the remaining steps. A fresh
agent can pick up from here. The project owner adjudicates design decisions;
the migrating agent implements per this document and the referenced artifacts.

## Current state (facts, verified)

- `nail` (legacy) is frozen at commit `ca59215` on `main` (pushed). All comments
  stripped from every `.rs` file (comment-stripper-rs, `--strip-docs`). The git
  history retains everything, including the old design docs, if ever needed.
- `nail_new` is a git repo (`main`), committed baseline containing:
  - skeleton per README §3 (three crates `nail_common`/`nail_back`/`nail_front`,
    edition 2024, empty layered modules, `configuration/`/`data/`/`log/`/`test/`
    with .gitkeep);
  - `document/reference/` — comment-stripped snapshot of the original code.
    WARNING: this is **untrusted legacy code** — it is full of bugs and
    compromise designs (32 known defects already adjudicated in
    `document/adjudication.md`). It is an inventory of what the old code
    *does*, NOT ground truth and NOT the spec. Regenerable via
    `git -C nail archive HEAD` + `comment-stripper-rs --strip-docs` +
    `cargo check`.
  - `document/reconstruction/` — reconstruct@2.17.0 output (redesign/complex/
    describe/tdd), **fully enriched**: `--check` PASSES;
  - `document/adjudication.md` — all 32 source-inconsistency items decided
    (18 fix / 7 remove / 7 keep).
- Reconstruction highlights: `features/01-project-setup/PRD.md` and
  `features/02-code/PRD.md` (zero unresolved callouts); `architecture/`
  INTERFACES.md (31 routes, verified), DATA-MODEL.md (37 entities, 24 enums),
  ARCHITECTURE.md (old→new layer mapping), 00-overview/PRD.md.
- Tooling present: node v22; `/home/qkun/reconstruct-tool` (v2.17.0);
  `comment-stripper-rs` in `~/.cargo/bin`; wasm32-unknown-unknown target.
- **Phase 2 (common crate) is COMPLETE** (2026-08-13): `nail_common` has the
  confirmed 9-module tree (`text, name, tag, response, hash, time, pow,
  request, search`), all implemented TDD red→green with 101 unit tests green
  at `test/unit/common/<module>/tests.rs` (wired via `#[path]`). Dependencies:
  anyhow, ascon-xof128, hex, pso-vdf, serde (derive), time (formatting), uuid
  (serde, v7); dev: serde_json. Owner refinements landed in
  `document/adjudication.md` (Phase 2 confirmations section): typed
  lowercase `DeleteMode`, unified `DeleteBody`, generic `err()`, panic-free
  `token() -> anyhow::Result`, both-or-neither check on `EmailReadRequest`,
  no tracing in common, RFC3339 formatter with configurable offset (item
  #19), `SearchHit` single-sourced with `SearchRange`-typed field.
  -  Engineering skills setup ran once: `AGENTS.md`, `docs/agents/*`, issue
  tracker = adjudication + handoff, domain docs at `document/context.md` +
  `document/adr/`.
- **HANDOVER (2026-08-13)**: implementation transfers from the Phase 2 agent
  to a new agent. Phase 2 commits: scaffold `1c8d543`, nine module slices
  `d40a9ac`..`9038b72`, handoff `22a4f3c`. `README.md` was amended by the
  owner AFTER the Phase 2 session — re-read it in full before Phase 3.
- **Phase 3 kickoff APPROVED (2026-08-13)**: `domain-modeling` (pin the
  vocabulary in `document/context.md`) → `codebase-design` for the back
  module tree (every boundary justified by the new layers' responsibilities
  and callers — no legacy-mirroring, README §4.1) → authentication/session
  slice with `tdd`. #26 is closed (query parameter). #5 remains an
  owner-confirm item at its slice.
- Common API contracts to honor at Phase 3 call sites: `now_ms() ->
  Result<u64, SystemTimeError>` (propagate with `?`), `uuidv7_timestamp_ms`
  returns `None` for non-v7 ids, `format_rfc3339_with_offset(utc_ms,
  offset_seconds)` accepts whole-minute offsets only (extremes ±23:59; `Z`
  for UTC).
- **Phase 3 slice 1 (authentication/session) COMPLETE (2026-08-13)**: the
  auth lifecycle is implemented fresh across the four layers and TDD-verified.
  Routes live: `GET /challenge/read`, `POST /email/read?intent=...`,
  `POST /user/create`, `GET /session/read`, `POST /session/delete`. Back module
  tree is ADR-0001; the `intent` contract is ADR-0002 (item #26, closed).
  Glossary at `document/context.md`. Back tests: 59 green; common tests: 104
  green (+3 for `EmailReadIntent`). `change_email`/`deregister` branches are
  stubbed to 400 "email intent is not supported yet" until slice 2.
  - Legacy comparison (README §8 gate, see below): new code is equal-or-better
    on all five axes; the gains are (a) a generic `TokenCache<E>` replacing
    the legacy `repo/token/*` six-file near-duplication, (b) an `EmailSender`
    trait + `RateLimitedSender`/`SmtpSender` seam so the email flow is testable
    in-process, (c) explicit `intent` removing the session-validity inference,
    (d) corrected error semantics where the PRD contradicted the legacy
    (garbage session token → 401 "invalid session" per AC5, not 400; email
    token redeem → one 400 "invalid or expired token" per AC4). No performance
    regression identified; the PoW/cache hot paths use the same moka/minroot
    building blocks.
  - Probe finding (recorded): moka's eviction listener runs via
    `run_pending_tasks()`/housekeeper, so the token reverse-index cleanup on
    eviction is eventually consistent — same as legacy. Verified deterministically
    via a capacity-eviction test that calls `run_pending_tasks()`.

## Rules for the migrating agent (non-negotiable)

Read `nail_new/README.md` in full first. Highlights:

- English-only everywhere (code, docs, comments, UI strings — item #14).
- No `mod.rs`; same-named file+folder module pairs; ≤512 lines/file,
  ≤256 lines/function, nesting ≤4.
- Panic-free: no unwrap/expect; errors propagate with `?`; error conversion
  only at layer boundaries; interface maps the final error into
  `{code, data, message}`.
- UUIDv7 ids; all hashing with ascon family; PoW = MinRoot VDF.
- No hardcoding — anything configurable lives in toml; timezone configurable
  (item #19).
- Dependency direction (mandatory): back `interface → logic → repository →
  infrastructure`; front `router → page → request → infrastructure`;
  `common` depends on nothing internal.
- Data structures first, then business logic (README §7).
- Dependencies: `cargo add`, alphabetical, latest non-conflicting (README §9).
- Testing: every function across all cases; exhaustive boundaries +
  randomized regular cases (README §12); TDD red→green (PRDs carry `--tdd`).
- Agent workflow (README §13): grep only via terminal; never the diagnostics
  tool; terminal `cd` must be `qkun/...`-prefixed or absolute under `/home/qkun`.
- Fresh module design (README §3 note + §4.1 + §4.2, added by owner): the
  module trees beneath the backend layers and the frontend layers must be
  designed fresh for `nail_new`; copying the legacy module division is
  absolutely forbidden; "the legacy code did it this way" is never an
  acceptable justification.
- Dead code (README §5.4, updated by owner 2026-08-13): do NOT chase dead or
  unused code during the migration — interim code may be consumed by later
  slices. Batch-remove all dead code as the FINAL task of Phase 5, then the
  zero-warning gate is enforced. "Never used" warnings are expected and fine
  until then.
- Legacy comparison + pre-study (README §8, updated by owner 2026-08-13):
  before implementing a strong-reference area (db / cache / email / API
  design) study the legacy implementation first; after each slice/phase,
  compare the new code vs the legacy on readability, correctness, elegance,
  conciseness, and performance, and report the comparison + any fixes to the
  owner.
- Comments (README §5.5, added by owner): code must be self-explanatory —
  zero or very few comments; comments only for non-obvious intent,
  constraints, or tradeoffs; never restate the code.
- Quality gate (README §8, added by owner): after each large module (a domain
  slice or a phase), compare the new code against the corresponding legacy
  code on readability, correctness, elegance, conciseness, and performance;
  ground the comparison in library source reading and probe tests wherever
  behavior or performance is in doubt; if inferior, weigh the fix cost,
  correct when worthwhile, re-run the full test suite, and report to the
  owner. Comparison only — the legacy code remains untrusted.
- Strong-reference areas (README §8, added by owner): before implementing the
  legacy's database design, cache design, email-sending business logic, or
  backend API design, read and study the legacy implementation carefully
  first — understand its reasoning before writing new code.
- Dead code (README §5.4, added by owner): do not chase dead or unused code
  during the migration; `#![allow(dead_code)]` at each crate root during the
  migration; batch-remove all dead code in one pass after the entire
  refactoring is complete (the final task of Phase 5), deleting the allow
  attributes with it.
- Handoff (README §13): update `document/handoff.md` at the end of every
  completed slice and phase, before reporting to the owner.
- Principle (README §8, added): facts from source and probes — probes outrank
  source, source outranks guessing; never guess.
- Skills (MANDATORY): before every covered task, invoke the matching skill via
  the `skill` tool and follow it — skipping a skill is a process violation
  (see the Skills — mandatory usage section).

## Reference materials

- `nail_new/README.md` — the constitution (read first).
- `nail_new/document/adjudication.md` — the 32 verdicts to implement.
- `nail_new/document/reconstruction/features/02-code/PRD.md` — the domain spec
  (user stories, numbered FRs tagged [confirmed]/[inferred], acceptance
  criteria, test plan).
- `nail_new/document/reconstruction/architecture/INTERFACES.md` — route
  contracts (auth/input/output per route).
- `nail_new/document/reconstruction/architecture/DATA-MODEL.md` — entities,
  relations, enums.
- `nail_new/document/reconstruction/architecture/ARCHITECTURE.md` — observed →
  target layer mapping.
- `nail_new/document/reference/code/` — the legacy code (see the WARNING in
  Current state: buggy, untrustworthy). Read it to understand what exists and
  to extract behavior, but never assume any line is correct (README §8:
  "Verify every line... do not assume any of it is correct"). When behavior is
  ambiguous, write a probe test to observe it.
- `nail` (frozen) — git archaeology if a detail is missing.

## What the target is (and is not)

- The target is **PRD requirements + adjudication verdicts + the README
  architecture rules**. The migration implements the *corrected* behavior, not
  a faithful copy of what the legacy code happens to do.
- The reconstruction PRDs document observed behavior — including the bugs.
  They are an inventory, not a spec. Every "[confirmed]" tag means "the old
  code does this", never "this is desired".
- 32 known defects/compromises are already adjudicated (fix/remove/keep). When
  the legacy code contradicts a verdict, the verdict wins.
- Anything new found during migration: probe it, then report to the owner —
  never silently preserve a bug or a compromise design.
- The principle (README §8): probes outrank source, source outranks guessing;
  facts are constructed from probes and source together. Two kinds of source,
  two stances:
  - **Library / dependency source is TRUSTED** (agdb, seekstorm, axum, moka,
    cedar-policy, leptos, pso-vdf, ascon, ...): read it to learn the real API;
    when a library misbehaves, look for the official solution in its source
    first — an apparent defect is usually a lack of familiarity with it
    (README §8). Probe tests confirm.
  - **The legacy `reference` code is DANGEROUS**: verify every line, never
    assume any of it is correct (README §8). Probes are the decisive tool
    wherever its behavior is in doubt.

## Remaining steps

### Phase 2 — common crate (contract layer, TDD) ✅ DONE

1. ✅ Placeholder modules renamed into the confirmed 9-module split
   (`text, name, tag, response, hash, time, pow, request, search`); module
   list confirmed by the owner before implementation (2026-08-13).
2. ✅ Implemented per DATA-MODEL.md + adjudication: unified `DeleteBody` with
   typed `DeleteMode` (drop the empty delete structs, #2/#11); single
   `SearchHit` in `common::search` with `SearchRange`-typed field (#10);
   English-only labels and messages (#14); dead request structs removed
   (`CheckEmailRequest`, `EmailUpdateSendRequest`, `EmailUpdateConfirmRequest`,
   `VerifySessionRequest`, `AuthorCheckRequest` — see #11). `EmailReadRequest`
   keeps `{pow?, old_email_pow?, new_email_pow?}` + a pure both-or-neither
   consistency check; #26 `intent` is a query parameter (confirmed, closed).
3. ✅ TDD per module, one slice one commit; seams agreed in the Phase 2
   proposal. Gate met: `cargo test` on `nail_common` all green (101 tests).

### Phase 3 — backend migration, one domain per slice

Per domain: read the domain slice of the 02-code PRD → write failing tests
first (red) → implement (green) → commit. Suggested order:

1. ✅ Authentication/session — challenge, emailed-token flow with the NEW
   `POST /email/read?intent=authenticate|change_email|deregister` contract
   (item #26; `intent` dispatch + `authenticate` branch live; `change_email`/
   `deregister` branches stub 400 until slice 2), session create/verify/delete.
2. User domain — item #15 (symmetric email_hash defaults, no swallowed
   errors), item #27 (idempotent deregister confirm).
3. Article + version — item #6 (gate version list), #16 (invalid id → 400),
   #17 (ContentHashExists variant), #18 (read_article aligned with
   consumption), #20 (no unconditional startup index rebuild; build when
   absent, maintain at write time), #21 (seekstorm count everywhere),
   #23 (drop redundant author lookup).
4. Comment domain — backend create/reply/list/update/delete with `DeleteBody`
   mode contract (item #2 backend already correct).
5. Download/PDF — item #1 (mint returns `.../content/read?token={token}`,
   consume uses `params.token`, single-use 60s, bound to user), item #28
   (hash-based filenames only; simplify sanitize), item #29 (document the
   mint-JSON contract).
6. Role/authorization — item #5 (DELETE visibility from agdb schema, Cedar
   schema/entities, and policy.cedar policy 2; **rewrite policy 2 to keep the
   de-facto read-open behavior: any authenticated principal may read
   article/version/comment — CONFIRM this semantics with the owner at this
   slice**), #7 (member_count computed or dropped), #8 (all REQUIRED_ROLES
   protected from delete), #9 (duplicate role → 400).
7. Config/email/infrastructure — item #13 (remove dead config fields),
   #19 (timezone from toml, served via `/config/read`), #22 (keep multipart
   read-then-validate; document), #26 (email service with explicit intent),
   #32 (e2e feature flag kept; test tree comes in Phase 5).

Layering per ARCHITECTURE.md: interface = HTTP + envelope + session-token +
PoW placement; logic = business rules (near-pure); repository = agdb + moka +
seekstorm; infrastructure = axum bootstrap, toml conf, SMTP client, PDF store,
tracing, Cedar engine.

### Phase 4 — frontend migration (after the backend API is stable)

- Layering per README §4.2 (router → page → request → infrastructure).
- Items: #14 (ALL UI strings English — full rewrite), #24 (page size from
  `/config/read`), #25 (has_next = page < total_pages), #3 (delete UI without
  per-comment pre-check; backend authoritative), #12 (remove the dead
  `/private/email/check` link), #4 (delete `pow-worker.js`; main-thread VDF).
- Leptos CSR, no CSS (README §10), compile-time `api_base_url`, runtime
  config from `/config/read` with compile-time fallback.
- Gate: `cargo check --target wasm32-unknown-unknown` on `nail_front`.

### Phase 5 — tests + end-to-end

- Rebuild the test tree per README §12: unit families at the four backend
  seams (interface/logic/repository/infrastructure) + common.
- e2e strategy (item #32): decide with the owner; the old design was a real
  process stack (back + pingap + chromium) with an in-process SMTP sink —
  see the old `test/` tree in `nail` git history (deleted from reference by
  owner choice).
- Gate: full `cargo test` suites; e2e serial (`--test-threads=1`) if ports are
  shared.
- Final batch dead-code cleanup (README §5.4): after the four phases, remove
  every dead/unused item in one dedicated pass, then re-run the full
  `cargo test` and `cargo check` with zero warnings. Mechanism: during the
  migration, `#![allow(dead_code)]` sits at each crate root (`nail_common` /
  `nail_back` / `nail_front`) so interim code compiles warning-free; the
  cleanup pass deletes the attributes together with the dead code and
  re-enforces the zero-warning gate.

## Ongoing discipline

- One task = one commit (`git add .` + commit + push) with an English message
  reflecting the actual change; archive before and after each task.
- Kill leftover backend/proxy processes after e2e (they lock the agdb file).
- Update `document/handoff.md` at the end of every completed slice and phase:
  current state, what was done, what is next, open items. Never finish a
  slice, a phase, or a session with a stale handoff.
- Keep `document/adjudication.md` current as decisions refine; record new
  contracts (item #26 intent param, item #5 policy change) as ADRs under
  `document/`.
- Dead-code removal is the FINAL task of Phase 5 (README §5.4), a single
  batch pass after the whole refactoring — not per-slice.

## Skills — mandatory usage

Skills are MANDATORY, not optional guidance: before every task covered by a
skill below, invoke it via the `skill` tool and follow its instructions.
Working without the matching skill is a process violation. The list below is
a checklist — every task type has a required skill, and each entry marks
when it MUST be invoked. The migrating agent has the same skill set as the
orchestrator. `nail_new/README.md` is the constitution and outranks any
skill.

### setup-matt-pocock-skills — run ONCE, before any engineering skill use

- **When**: first session in nail_new, before tdd / codebase-design / etc.
- **What**: configures the repo for the engineering skills: issue tracker
  (local markdown under `.scratch/` — no remote exists), `AGENTS.md`,
  `CONTEXT.md` + `docs/adr/` layout, triage labels.
- **Here**: gives tdd / to-spec / domain-modeling their landing spots
  (CONTEXT.md, ADRs, issue tracker).

### tdd — every implementation slice

- **When**: Phase 2 (each common module), Phase 3 (each domain slice), Phase 4
  (pure logic parts), Phase 5 (test rebuild). Every red → green cycle.
- **What**: agree the seams with the owner first; one failing test → minimal
  implementation → repeat. No horizontal slicing (all tests first). Tests
  verify behavior through public interfaces, not internals.
- **Here**: the PRDs carry `--tdd` build order; write the failing acceptance
  tests first per slice; the PRD requirements + adjudication log are the
  expected-behavior source, not the legacy code.

### codebase-design — every module boundary decision

- **When**: before designing each layer's module tree (common modules; back
  interface / logic / repository / infrastructure; front layers).
- **What**: deep-module vocabulary (interface, depth, seam, adapter, leverage,
  locality) to decide where seams go and how deep modules should be.
- **Here**: README §4 fixes dependency direction only; this skill decides
  module depth/shape and the test seams at each boundary.

### diagnosing-bugs — whenever behavior contradicts expectations

- **When**: PRD behavior contradicts the reference source; a library misbehaves
  (check its trusted source first, README §8); a test fails for unknown
  reasons; one of the 32 adjudication items surfaces during implementation.
- **What**: reproduce → isolate → root cause → fix. Evidence and logging
  first; no speculative edits.
- **Here**: the reference code is dangerous (bugs + compromise designs); treat
  every contradiction as a candidate bug — probe it, then report to the
  owner; never silently copy legacy behavior.

### domain-modeling — pin the vocabulary

- **When**: once at the start of Phase 3 (or Phase 2), then whenever a new
  term appears (e.g. the #26 `intent` values).
- **What**: define the ubiquitous language; record it in CONTEXT.md; add ADRs
  for decisions.
- **Here**: article / version / comment / session / role / recycler /
  transfer / hard / intent must mean the same thing across code, tests,
  PRDs, and docs.

### grilling — stress-test a plan/decision with the owner

- **When**: before implementing an open or risky decision (#5 read-open
  semantics, #26 intent contract), or when the agent is unsure of a design
  direction. Do not grill routine work.
- **What**: one-question-at-a-time interview (recommend an answer each round)
  until the decision is sharp.
- **Here**: use for the two owner-confirm items in Phase 3.

### grill-with-docs — grilling that also writes docs

- **When**: before implementing the two owner-confirm items (#5 read-open
  semantics, #26 intent contract), when the outcome should land as ADRs +
  glossary. Do not grill routine work.
- **What**: one-question-at-a-time interview (recommend an answer each round)
  + writes CONTEXT.md glossary + ADRs as it goes.
- **Here**: produce the ADR for #26 (new `/email/read?intent=` contract) and
  #5 (visibility removal + policy 2 rewrite).

### handoff — end of every slice, phase, and session

- **When**: at the end of every completed slice, phase, and session (before
  another agent takes over).
- **What**: compact the session into a handoff document; reference artifacts
  by path; do not duplicate content already captured elsewhere.
- **Here**: update/extend `document/handoff.md` (or write a session-specific
  handoff); keep `document/adjudication.md` current.

### to-spec — formalize discussed decisions

- **When**: after a grilling/decision round, if the owner wants a formal spec
  in the issue tracker.
- **What**: synthesize the conversation into a spec and publish it to the
  issue tracker (`.scratch/` once setup ran).
- **Here**: optional — the PRDs + adjudication already act as specs; use only
  for new contracts worth formalizing (#26, #5).

### improve-codebase-architecture — scan for deepening opportunities

- **When**: when a migration slice feels tangled despite codebase-design, or
  when the owner asks for an architecture health check on the new code.
- **What**: scans a codebase for deepening opportunities and presents them as
  a visual report, then grills through whichever one is picked.
- **Here**: not needed during the build (the architecture is fixed by the
  README); useful later as a periodic health check on nail_new. If ever run
  against `nail`, do not migrate its proposals without owner review.

### create-skill — authoring new skills

- **When**: only if the owner asks to create or patch an agent skill.
- **What**: guides the creation of a new SKILL.md with instructions.
- **Here**: skip unless the owner requests it.
