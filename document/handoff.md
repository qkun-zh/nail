# Handoff — nail → nail_new migration

Pickup doc for the current migrating agent. Rules live in `nail_new/README.md`
(the constitution — read it in full first). Full history, per-slice §8.3 gates,
and probe findings: `document/progress-log.md`. Adjudication verdicts:
`document/adjudication.md` (32 + #33 + #34).

## Current state (facts)

- `nail` (legacy) frozen at `ca59215`; `document/reference/` = comment-stripped
  snapshot, **untrusted** (33 adjudicated defects). Reconstruction docs pass
  `--check`; PRD `features/02-code/PRD.md` = the domain spec; INTERFACES /
  DATA-MODEL / ARCHITECTURE under `document/reconstruction/architecture/`.
- **Phase 3 backend: DONE (slices 1-7); typed-DTO sweep: DONE.** back **245
  tests green** (common 108), `cargo check` zero warnings. History:
  `document/progress-log.md`.
- **Phase 4 frontend (Leptos CSR): DONE.** nail_front **61 unit tests green**,
  `cargo check --target wasm32-unknown-unknown` and host `cargo check` both
  **zero warnings**; back 245 + common 108 still green (no regression). Working
  tree clean at `99b83c7`. No git remote. Details below.
- Backend layering per ADR-0001; `intent` per ADR-0002; glossary
  `document/context.md`; Cedar engine landed (slice 6); `/config/read` returns
  the typed `common::response::RuntimeLimits` (slice 7).
- `_`-prefixed archive files removed (`aa39cb6`); pre-rewrite code in git
  (`8de3490^`). File tools work on all paths (verified). The
  `thermo-nuclear-code-quality-review` skill is AVAILABLE (fixed 2026-08-14:
  its `disable-model-invocation: true` flag excluded it from the registry — a
  known harness bug; the flag was removed). Use it for §8.3 gates; its 1k-line
  default bar is superseded by README §5.3 (512 lines).

## Pending — current agent

1. **Phase 4 — frontend migration: DONE.** Layering per README §4.2; Leptos
   CSR, no CSS; runtime config from `/config/read` (typed `RuntimeLimits` as
   the limits signal) with compile-time fallback; all adjudication items for
   the frontend implemented (#14 English UI, #24 page size from config, #25
   `has_next = page < total_pages` from the server flag, #3 no per-comment
   pre-check with the backend authoritative, #12 no `/private/email/check`, #4
   no worker file — PoW runs in-wasm on the main thread, #19 timezone via
   `RuntimeLimits.timezone_offset_seconds`). Gate met: `cargo check --target
   wasm32-unknown-unknown` zero warnings on `nail_front`.
2. **Phase 5 — remaining: tests + e2e + cleanup** (next). Rebuild the remaining
   test tree per README §12; e2e strategy (#32) with the owner; final batch
   dead-code cleanup (README §5.4) in one pass, then zero-warning gate.

## Phase 4 — what was built

Composition root `code/front/src/main.rs` (config fail-fast on invalid
compile-time scheme → limits signal + notification system + session state →
mount `AppRouter` + `ToastContainer`). Module layout (each module = same-named
`.rs` + folder; no `mod.rs`):

- `infrastructure/` — `config` (embedded `configuration/front.toml`,
  `api_base_url`; empty = same-origin), `limits` (RuntimeLimits signal,
  compile-time defaults, per-field zero fallback; timezone 0 = UTC kept),
  `storage` (localStorage read/write/remove), `pow` (in-wasm
  `common::pow::prove` adapter).
- `request/` — `error`, `url` (encodeURIComponent-equivalent path/query
  building), `envelope` (parse + unwrap), `session` (token
  get/set/clear + 401 invalidation hook), `http` (fetch core: 30 s abort,
  session-token header, envelope unwrap, 401 clears token + notifies),
  `pow` (prove_pow = fresh challenge + prove), `auth`, `user`, `article`,
  `version`, `comment`, `download` (mint + same-origin-guarded consume +
  blob/anchor save + Content-Disposition filename). All 21 reached routes are
  exercised; responses deserialize the `common::response` DTOs directly.
- `page/` — `session_gate` (checking/authenticated/anon; `RootGate` renders
  `<Outlet/>` or "who are you?" — applied at the router root, `/private/
  authenticate` exempt), `author_gate` (is_author re-check with sequence
  guard), `notify` (toasts 5 s/3 s, countdown, dismiss, history cap 100),
  `pagination` (page clamp + server-`has_next`-driven prev/next), `validation`
  (mirrors common name/tag/text rules + PDF MIME/name/size sniff),
  `time_format` (#19 via limits offset), `draft` (FR-66 query-param draft
  persistence), `index`, `not_found`, `public/` (index, article
  index/search/create/detail/update/delete, version list/create/detail,
  comment index/reply/delete), `private/` (index, authenticate, name,
  name/update, email, email/update, logout, deregister).
- `router.rs` — path → page map only (FR-60), ParentRoute nesting, fallback
  NotFound. `/private/authenticate` is declared outside the root `RootGate`
  `ParentRoute`, so it always renders ungated (FR-61).

Frontend unit tests (61) live at `test/unit/front/<module>/tests.rs` wired via
`#[path]` from each module: envelope unwrap, 401 session invalidation, url
encoding, download same-origin guard, content-disposition filename,
pagination clamping, client validation parity, draft query building, timezone
formatting, notify durations/countdown/history cap, limits fallbacks, config
scheme validation, PoW prove adapter. Leptos view components stay thin; pure
logic is host-testable.

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
- **Responses are fixed data structures, not `json!` (owner, 2026-08-14)** —
  done before Phase 4; the Phase 4 frontend consumes the shared DTOs directly.
- **Search split into request/response (owner, 2026-08-14)** — done:
  `ArticleSearchParams` in `common::request`; `SearchHit`/`SearchArticleItem`/
  `SearchPage` in `common::response/search`; shared enums stay in
  `common::search`. **#34**: `has_more` → `has_next`.
- **Pagination config scope (owner, 2026-08-14)**: config holds only
  frontend-facing `search_page_size` + `max_search_pages`; the backend clamp
  caps `max_search_page_size` (200) and `max_page` (10000) stay hardcoded
  constants.
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

- `README.md` — constitution. `document/adjudication.md` — 34 verdicts.
  `document/context.md` — glossary. `document/adr/` — 0001, 0002.
  `document/progress-log.md` — history + §8.3 gates. `document/dsh-cli.md` —
  the agent runner (DeepSeek Harness CLI) setup, config, commands.
- `document/reconstruction/features/02-code/PRD.md` — the spec (FR-1..66).
- `document/reconstruction/architecture/` — INTERFACES (31 routes),
  DATA-MODEL, ARCHITECTURE.
- `document/reference/code/` — untrusted legacy; probe to verify. `nail`
  (frozen) — git archaeology.

## Remaining steps

### Phase 5 — remaining: tests + e2e + cleanup

After Phase 4: rebuild the remaining test tree per README §12; e2e strategy
(#32) with the owner; final batch dead-code cleanup (README §5.4) in one
pass, then zero-warning gate.

**E2E tooling facts (owner, 2026-08-14)**: the legacy e2e stack is back
process + pingap + chromium with an in-process SMTP sink
(`test/end_to_end/browser/context.rs`). Chromium is already installed
(`/usr/bin/chromium`). For the browser driver use the **latest chromiumoxide**
crate version (NOT the legacy 0.9.1); add it `cargo add` as an optional,
feature-gated (`end_to_end`) dependency when Phase 5 starts — its crates are
already cached in the local registry. **pingap is NOT installed on this
machine**; it has a GitHub repository with released binaries — obtain the
release binary from GitHub when the e2e phase starts.

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
| thermo-nuclear-code-quality-review | §8.3 gate per slice | available since 2026-08-14 (flag removed); its 1k-line default bar is superseded by README §5.3 (512 lines) |
| setup-matt-pocock-skills | first session, once | already done |
