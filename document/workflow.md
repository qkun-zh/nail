# Workflow — the correctness loop

Mandatory for every code change. Authority: `README.md` (constitution),
`AGENTS.md` (tool/skill rules). This file wins on execution order.

## Loop

```
execute(R):
  1  baseline green          # bugfix: failing repro first, else STOP
  2  clean tree              # dirty → commit unrelated or ask
  3  pin(R)                  # precise, consistent, no ambiguity
  4  plan(R)                 # ordered, verifiable slices with exit tests
  5  exec_doc(R, plan)       # single source of truth — §Exec doc
  6  research(plan)          # source + probe double evidence — §Evidence
  7  gate adoption           # evidence consistent + user approves — §Gate
  8  for slice in plan: red → green → gate → commit → push → CI green
  9  handoff                 # record state, report
 10  final gate              # CI green: tests + clippy + fmt; never red
```

---

## §1 Baseline

- All tests green → proceed.
- Bug fix → write a failing repro test first; must fail before anything else.
- Tests red (not a bug fix) → STOP. Fix tree before proceeding.

## §2 Clean tree

Uncommitted changes unrelated to R → commit them or ask user. Never discard.

## §3 Pin requirements

R must be unambiguous and internally consistent. Any ambiguity → ask user.
Outcome: one precise, testable requirement statement.

## §4 Plan

Ordered list of slices. Each slice states:

- **Goal** — one sentence.
- **Files** — exact files to touch.
- **Red** — test that must fail before implementation.
- **Green** — expected behavior after implementation.
- **Exit test** — command proving slice complete.

Unknowns → §6; they don't block planning but must be flagged.

## §5 Exec doc

Write `document/exec/<4-char code>_<slug>.md` — 4-character random
alphanumeric code; unique, no reuse (agent invents it, no scripts/system
calls). Under 300 lines.

Deleted once task fully complete (see §10).

**Required sections** (empty only with explicit "N/A" + one-line reason):

1. **Requirement** — pinned R, acceptance criteria.
2. **Scope** — in-scope and explicitly out-of-scope.
3. **Design decisions** — modules touched, seam choices, trade-offs, rationale.
4. **Slice breakdown** — from §4, one entry per slice.
5. **Open unknowns** — evidence source for each (source/probe).
6. **Verification plan** — which dimensions per slice, how verified.
7. **Risks** — what could go wrong, mitigation, rollback.
8. **Constraints** — task-specific prohibitions (e.g. "don't touch X").
9. **Questions** — unresolved ambiguities for user.

Update in-place when evidence contradicts; append `## Change log` at bottom.
Single source of truth during execution — read at start of every slice.

## §6 Evidence

Every unknown gets **source** (pinned lib/repo read) + **probe** (disposable
test). `source ≠ probe` → resolve before proceeding. Contradicts R → ask.

**When evidence is mandatory:**

- Return/side-effect unclear → source + probe.
- Source contradicts belief → probe wins.
- Two APIs look equivalent → probe to choose.
- Behavior visible in source → source suffices.

**Verification dimensions** — each applicable dimension must be `verified` or
`N/A + reason`; `unknown` → back to §6. Unevidenced dimension blocks gate.

| Dimension | Check |
| --- | --- |
| Correctness | behavior matches R, normal + edge cases |
| Behavior change | input/output delta vs baseline = R |
| Time complexity | Big-O of touched path |
| Space complexity | allocations, cache, DB footprint |
| Performance | latency/throughput delta vs baseline |

**Reuse before build** — use existing official APIs first. Custom wheel or
workaround needs explicit user consent + recorded rejection reasons.

### Probe file layout

One file per probe, never a shared `probe.rs`.

- Location: `test/unit/{common,back,front}/<area>/probe_<NNN>_<purpose>.rs`
- `<NNN>`: 3-digit zero-padded, unique across repo, lowest-first.
- First line: doc comment with purpose, source evidence, acceptance question.
- Function name: `probe_<NNN>_<purpose>`.
- One agent per number; never edit another agent's probe file.
- After gate: promote (rename, drop `NNN_`, move) or delete.

## §7 Gate (adoption)

No code until:

1. Evidence consistent across all applicable dimensions.
2. User explicitly adopts the plan.

Evidence or user rejects → back to §3.

## §8 Slice loop

Per slice:

```
red:   write test → cargo test → must fail
green: implement → cargo test → must pass
gate:  fmt --check && clippy -D warnings → clean
commit: one commit per slice, clean tree
push:  git push origin main → CI runs all tests (see document/run.md)
       confirm with document/ci-watch.sh; failing CI job = failed gate
```

The test gate is the CI run, not the local machine — see `document/run.md`
(Testing (CI-first)) for commands and the push/CI-check procedure. Local
`cargo test` is a smoke pass only.

Gate fails → debug, fix, re-gate. Never skip gate.

**Resource contention** — prefer CI over local builds; before any local
`cargo` build/test (here, §9, or probe tests), check machine load (`uptime` /
`ps -eo pcpu` / `mpstat`). Heavily loaded → back off, poll periodically. Never
build on busy machine; shared tree means unreliable results or disrupted runs.

## §9 Handoff

Leave work for another agent to pick up, so next session needs no memory of
this one. Mandatory before reporting done.

Update `document/handoff/readme.md` (and per-task file) per its task
organization rules: current state, slices done, decisions made, remaining
risks. Drop completed slices; keep only incomplete/in-progress entries. Label
task ownership; never touch others' tasks. Use the 64-em-dash divider.
Incomplete or stale handoff → red gate; never report green without one.

When task fully complete, delete its exec doc
(`document/exec/<4-char code>_<slug>.md`), so only in-progress exec docs
remain.

Report to user.

## §10 Final gate

Full build + all tests + clippy (0 warnings) + fmt, as gated by CI: push
`main` and confirm `document/ci-watch.sh` reports success (see
`document/run.md`). Must reproduce green. Never report red. Gate fails → back
to §8.

---

## Loop-back

| Condition | Return to |
| --- | --- |
| Exec doc incomplete / exceeds 300 lines | §4 |
| Evidence contradicts R | §3 |
| User rejects adoption | §3 |
| source ≠ probe | §6 |
| Research improves plan | §4 |
| Scope changes | §4 |
| Bug repro passes (no bug) | §3 |
| Test fails unexpectedly | §6 |
| Slice gate fails | §8 |
| Final gate fails | §8 |
| Requirement changes | §3 |

## STOP

At any phase, may STOP and report: leave tree as-is, state blocked phase +
reason. Never proceed past a block on a guess.

---

## Invariants

1. Every uncertainty is a question, never a guess.
2. No code without exec doc (§5) + evidence (§6) + adoption (§7).
3. API behavior from source + probe; official APIs win; wheel/patch needs consent.
4. Verification dimensions evidenced or N/A before every gate.
5. One commit per slice; clean tree at every loop-back.
6. Results reproducible by re-running gates.
7. No hand-edited `Cargo.lock`; no `unwrap`/`expect`; no secrets.
8. Never discard work; recover from commit or ask.
