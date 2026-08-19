# Handoff

## Task X: final cleanup + final gate

**Owner**: sub-agent (Task X of REFACTOR_PLAN)
**Exec doc**: `document/exec/J8wn_final-cleanup-gate.md`
**Status**: IN PROGRESS — plan written, awaiting adoption gate.

### Slices

- 0. Docs: exec doc `J8wn_final-cleanup-gate.md` + this handoff. IN PROGRESS.
- 1. Front allows: js.rs (delete, predicted dead) + state.rs (restore+document, predicted live). PENDING (gate).
- 2. Back allows (principal.rs, predicted live) + OffsetTime (Q1 pending orchestrator). PENDING (gate).
- 3. rustc-ice dumps in code/back (Q2 pending orchestrator). PENDING (gate).
- 4. Final gate: full matrix (common 117, back 583 per-module, front 82 + check + trunk; fmt/clippy all). PENDING (gate).

### Decisions / notes

- Evidence gathered pre-gate: `OffsetTime.offset` is READ at
  `logging.rs:40` (`self.offset` in `format_time`) — NOT compiler-dead;
  invariant `UtcOffset::UTC`. Deleting only allowed if orchestrator approves
  explicit invariant-field removal (Q1); recommendation: skip (strict
  "verified-dead only" reading).
- 4 `#[allow(...)]`s total in `code/`: principal.rs:27-28 (predicted live,
  f39c9c2 history), js.rs:1 (predicted dead — restriction-group lints not
  enabled; only `pedantic=deny` configured), state.rs:215 (predicted live —
  pedantic `too_many_arguments`, 9 params > 7).
- Sweep: 467 pub items in `code/*/src`; every one has >=2 references. No
  other dead items found. clippy 0 warnings (stable + nightly) on all crates
  at baseline.
- 2 untracked rustc ICE dumps in `code/back/` (2026-08-19, OOM-era); deletion
  pending orchestrator decision (Q2).
- Back tests MUST run per-module (`configuration_`/`infrastructure_`/`logic_`/
  `repository_`/`http_`) — single-process full suite OOMs on this 9GB box.

### Open questions

- Q1: delete `OffsetTime.offset` as invariant-field removal (orchestrator
  decision; evidence says not verified-dead)?
- Q2: delete the untracked rustc-ice dumps in `code/back/`?

————————————————————————————————————————————————