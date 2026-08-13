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
- Principle (README §8, added): facts from source and probes — probes outrank
  source, source outranks guessing; never guess.

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

### Phase 2 — common crate (contract layer, TDD)

1. Rename the `xxx`/`yyy`/`zzz` placeholder modules into the real shared
   modules (README §7: data structures first). Candidate split from
   DATA-MODEL.md: response envelope, request payloads, pow, hash (ascon),
   time, name/tag/text validation, search shapes. Confirm the module list with
   the owner before implementing.
2. Implement per DATA-MODEL.md + adjudication: unified `DeleteBody` (drop the
   empty `DeleteArticleRequest`/`DeleteCommentRequest`, item #2/#11); single
   `SearchHit` source of truth (item #10); English-only (item #14); no dead
   structs (items #10/#11).
3. TDD per module (tdd skill; agree seams first). Gate: `cargo test`
   on `nail_common` all green.

### Phase 3 — backend migration, one domain per slice

Per domain: read the domain slice of the 02-code PRD → write failing tests
first (red) → implement (green) → commit. Suggested order:

1. Authentication/session — challenge, emailed-token flow with the NEW
   `POST /email/read?intent=authenticate|change_email|deregister` contract
   (item #26), session create/verify/delete.
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

## Ongoing discipline

- One task = one commit (`git add .` + commit + push) with an English message
  reflecting the actual change; archive before and after each task.
- Kill leftover backend/proxy processes after e2e (they lock the agdb file).
- Keep `document/adjudication.md` current as decisions refine; record new
  contracts (item #26 intent param, item #5 policy change) as ADRs under
  `document/`.

## Skill usage guide

The migrating agent has the same skill set as the orchestrator. Invoke a skill
via the `skill` tool BEFORE the task it covers, and follow its instructions.
`nail_new/README.md` is the constitution and outranks any skill.

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

### grill-with-docs — grilling that also writes docs

- **When**: before implementing the two owner-confirm items (#5 read-open
  semantics, #26 intent contract), when the outcome should land as ADRs +
  glossary. Do not grill routine work.
- **What**: one-question-at-a-time interview (recommend an answer each round)
  + writes CONTEXT.md glossary + ADRs as it goes.
- **Here**: produce the ADR for #26 (new `/email/read?intent=` contract) and
  #5 (visibility removal + policy 2 rewrite).

### handoff — end of every session

- **When**: session end, before another agent takes over.
- **What**: compact the session into a handoff document; reference artifacts
  by path; do not duplicate content already captured elsewhere.
- **Here**: update/extend `document/handoff.md` (or write a session-specific
  handoff); keep `document/adjudication.md` current.
