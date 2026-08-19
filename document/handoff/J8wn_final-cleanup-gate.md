# Handoff

## Task X: final cleanup + final gate

**Owner**: sub-agent (Task X of REFACTOR_PLAN)
**Exec doc**: `document/exec/J8wn_final-cleanup-gate.md`
**Status**: DONE — all slices complete; FINAL GATE green (matrix below).

### Slices

- 0. Docs: exec doc `J8wn_final-cleanup-gate.md` + this handoff. DONE (`1ff295e`).
- 1. Front allows: probe-removed js.rs + state.rs allows; nightly clippy FIRED both (cast lints are in pedantic on clippy 1.99 nightly; too_many_arguments 9/7) → both RESTORED per protocol. No net code change. DONE (docs `d04b2dd`).
- 2. Back allows + OffsetTime: probe-removed both principal.rs allows; nightly FIRED `unused_async_trait_impl`; stable errored `unknown lint` without `unknown_lints` → both RESTORED (both live, one per toolchain). OffsetTime SKIPPED per Q1 (false positive). Bonus stable probes back/front 0 warnings. No net code change. DONE (docs `d25dc19`).
- 3. rustc-ice dumps: 2 untracked files deleted per Q2 approval (plain rm, no git op). DONE.
- 4. FINAL GATE — all green, full matrix below. DONE.

### Final gate matrix (2026-08-20)

| crate | tests | fmt --check | clippy nightly | clippy stable | build |
|---|---|---|---|---|---|
| common | 117/117 | clean | 0 warnings | 0 warnings | — (lib) |
| back | 583/583 per-module | clean | 0 warnings | 0 warnings | tests build bin |
| front | 82/82 | clean | 0 warnings | 0 warnings (wasm) | `cargo +nightly check` + `trunk build` OK |

Back per-module runs (each run reports binary total 583): configuration 11,
infrastructure 45, logic 282, repository 108, http 139. The +1s in
logic/repository are ONE test double-matched by both substring filters
(`back_tests::logic_session::hash_canonical_token_matches_the_repository_token_key`);
disjoint split = 11/45/281/107/139 = 583 — matches the brief exactly.

Health checks: backend `curl /config/read` OK, proxy `/api/config/read` OK.

### Decisions / notes

- Evidence gathered pre-gate: `OffsetTime.offset` is READ at
  `logging.rs:40` (`self.offset` in `format_time`) — NOT compiler-dead;
  invariant `UtcOffset::UTC`. ORCHESTRATOR DECISION (Q1): SKIP — field is
  read; the REFACTOR_PLAN "dead field" flag was a false positive. Recorded;
  no code change.
- 4 `#[allow(...)]`s total in `code/`: principal.rs:27-28 (LIVE on both
  toolchains: nightly fires `unused_async_trait_impl`; stable errors `unknown
  lint` without `unknown_lints` — probe-verified; restored), js.rs:1 (LIVE —
  cast lints are in pedantic on clippy 1.99 nightly; restored), state.rs:215
  (LIVE — too_many_arguments 9/7; restored).
- Probe protocol verdicts contradict the pre-gate predictions for js.rs
  (predicted dead, actually live): on clippy 1.99 nightly, `-D clippy::pedantic`
  implies `cast_sign_loss`/`cast_possible_truncation`. Empirical result is
  authoritative; allows stay with this rationale.
- Sweep: 467 pub items in `code/*/src`; every one has >=2 references. No
  other dead items found. clippy 0 warnings (nightly + cranelift) on all
  crates at baseline. Accepted by orchestrator.
- 2 untracked rustc ICE dumps in `code/back/` (2026-08-19, OOM-era);
  ORCHESTRATOR DECISION (Q2): delete (transient crash garbage, not work).
  Untracked → plain file removal, no git op.
- Stable clippy cannot run with the mandated `-Zcodegen-backend=cranelift`
  flags ("option Z is only accepted on the nightly compiler"); nightly is the
  project gate toolchain.
- Back tests MUST run per-module (`configuration_`/`infrastructure_`/`logic_`/
  `repository_`/`http_`) — single-process full suite OOMs on this 9GB box.
- Final result: Task X produced NO net code change (all 4 allows verified
  live and restored; OffsetTime false positive documented; dumps deleted).
  Project refactor scope complete; deferred items remain: D4 (repository
  response-assembly → logic), single-variant `_sync` renames, 404-vs-500
  semantics, ArticleListItem collapse, read_tags Option unify, delete_tag → ()

### Open questions

- None (Q1/Q2 resolved by orchestrator).

————————————————————————————————————————————————