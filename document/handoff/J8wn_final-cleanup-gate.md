# Handoff

## Task X: final cleanup + final gate

**Owner**: sub-agent (Task X of REFACTOR_PLAN)
**Exec doc**: `document/exec/J8wn_final-cleanup-gate.md`
**Status**: IN PROGRESS — slice 1 done (no net change), slice 2 next.

### Slices

- 0. Docs: exec doc `J8wn_final-cleanup-gate.md` + this handoff. DONE (`1ff295e`).
- 1. Front allows: probe-removed js.rs + state.rs allows; nightly clippy FIRED both (cast lints are in pedantic on clippy 1.99 nightly; too_many_arguments 9/7) → both RESTORED per protocol. No net code change; documented in exec change log. DONE (docs commit in progress).
- 2. Back allows (principal.rs, predicted live) + OffsetTime Q1 = SKIP (approved; false positive — field read at logging.rs:40). PENDING.
- 3. rustc-ice dumps in code/back — Q2 APPROVED: delete. PENDING.
- 4. Final gate: full matrix (common 117, back 583 per-module, front 82 + check + trunk; fmt/clippy all). PENDING (gate).

### Decisions / notes

- Evidence gathered pre-gate: `OffsetTime.offset` is READ at
  `logging.rs:40` (`self.offset` in `format_time`) — NOT compiler-dead;
  invariant `UtcOffset::UTC`. ORCHESTRATOR DECISION (Q1): SKIP — field is
  read; the REFACTOR_PLAN "dead field" flag was a false positive. Recorded;
  no code change.
- 4 `#[allow(...)]`s total in `code/`: principal.rs:27-28 (probe pending),
  js.rs:1 (LIVE — cast lints are in pedantic on clippy 1.99 nightly; restored),
  state.rs:215 (LIVE — too_many_arguments 9/7; restored).
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

### Open questions

- None (Q1/Q2 resolved by orchestrator).

————————————————————————————————————————————————