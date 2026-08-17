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
 5  exec_doc(R, plan)       # single source of truth — §Execution doc
 6  research(plan)          # source + probe double evidence — §Evidence
 7  gate adoption           # evidence consistent + user approves — §Gate
 8  for slice in plan: red → green → gate → commit
 9  final gate              # full build + all tests + clippy + fmt; never report red
10  handoff                 # record state, report
```

---

## Phase 1 — Baseline

| Situation | Action |
| --- | --- |
| All tests green | Proceed. |
| Bug fix | Write a failing repro test first; it must fail before anything else. |
| Tests red (not a bug fix) | STOP. Fix the tree before proceeding. |

## Phase 2 — Clean tree

Uncommitted changes unrelated to R → commit them or ask user. Never discard.

## Phase 3 — Pin requirements

R must be unambiguous and internally consistent. Any ambiguity → ask user.
Outcome: a single, precise, testable requirement statement.

## Phase 4 — Plan

Ordered list of slices. Each slice states:

- **Goal** — one sentence.
- **Files** — exact list of files to touch.
- **Red** — the test that must fail before implementation.
- **Green** — the expected behavior after implementation.
- **Exit test** — command that proves the slice complete.

Unknowns go to §Evidence (phase 6); they do not block planning but must be
flagged.

## Phase 5 — Execution doc

Write `document/exec/<4-char code>_<slug>.md` (4-character random
alphanumeric code; unique, no reuse). Under 300 lines.

The exec doc is deleted once its task is fully complete (see §Phase 10).

**Required sections** (empty only with explicit "N/A" + one-line reason):

1. **Requirement** — pinned R, acceptance criteria.
2. **Scope** — in-scope and explicitly out-of-scope.
3. **Design decisions** — modules touched, seam choices, trade-offs, rationale.
4. **Slice breakdown** — from §Plan, one entry per slice.
5. **Open unknowns** — evidence source for each (source/probe).
6. **Verification plan** — which dimensions per slice, how verified.
7. **Risks** — what could go wrong, mitigation, rollback.
8. **Constraints** — task-specific prohibitions (e.g. "don't touch X").
9. **Questions** — unresolved ambiguities for user.

Update in-place when evidence contradicts; append `## Change log` at bottom.
The exec doc is the single source of truth during execution — read it at the
start of every slice.

## Phase 6 — Evidence

Every unknown gets **source** (pinned lib/repo read) + **probe** (disposable
test). `source ≠ probe` → resolve before proceeding. Contradicts R → ask.

**When evidence is mandatory:**

| Situation | Minimum |
| --- | --- |
| Return/side-effect unclear | source + probe |
| Source contradicts belief | probe wins |
| Two APIs look equivalent | probe to choose |
| Behavior visible in source | source suffices |

**Verification dimensions** — each applicable dimension must be `verified` or
`N/A + reason`; `unknown` → back to phase 6. Unevidenced dimension blocks gate.

| Dimension | What to check |
| --- | --- |
| Correctness | behavior matches R, normal + edge cases |
| Behavior change | input/output delta vs baseline = R |
| Time complexity | Big-O of touched path |
| Space complexity | allocations, cache, DB footprint |
| Performance | latency/throughput delta vs baseline |

**Reuse before build** — use existing official APIs first. Custom wheel or
workaround needs explicit user consent + recorded rejection reasons.

### Probe file layout (concurrent-safe)

One file per probe, never a shared `probe.rs`.

- Location: `test/{common,back,front}/<area>/probe_<NNN>_<purpose>.rs`
- `<NNN>`: 3-digit zero-padded, unique across repo, lowest-first.
- First line: doc comment with purpose, source evidence, acceptance question.
- Function name: `probe_<NNN>_<purpose>`.
- One agent per number; never edit another agent's probe file.
- After gate: promote (rename, drop `NNN_`, move) or delete.

## Phase 7 — Gate (adoption)

No code until:

1. Evidence consistent across all applicable dimensions.
2. User explicitly adopts the plan.

Evidence or user rejects → back to phase 3.

## Phase 8 — Slice loop

Per slice:

```
red:  write test → cargo test → must fail
green: implement → cargo test → must pass
gate:  cargo fmt --check && cargo clippy -D warnings && cargo test → all pass
commit: one commit per slice, clean tree
```

Gate fails → debug, fix, re-gate. Never skip gate.

### Resource contention (before any build/test)

Before running any `cargo` build/test (here, §9, or probe tests), check the
machine load (`uptime` / `ps -eo pcpu` / `mpstat`). If the machine is heavily
loaded — likely another agent's compile/test in progress — **back off**: wait
(poll periodically) until load drops enough to run your own build without
contending. Never start a build on top of a busy machine; a shared tree means
results may be unreliable or someone else's run may be disrupted.

## Phase 9 — Final gate

Full build + all tests + clippy (0 warnings) + fmt. Must reproduce green.
Never report red. Gate fails → back to phase 8.

## Phase 10 — Handoff

Purpose: leave the work for another agent to pick up, so the next session
needs no memory of this one. Mandatory before reporting done.

Update `document/handoff/readme.md` (and the per-task file) strictly per its
**Task organization rules**:
current state, slices done, decisions made, remaining risks; drop completed
slices; keep only incomplete/in-progress entries; label task ownership; never
touch others' tasks; use the 64-em-dash divider. Follow the `handoff` skill
(AGENTS.md) and invoke it when wrapping up. A handoff that is incomplete or
stale is a red gate — never report green without one.

When a task is fully complete, delete its exec doc
(`document/exec/<4-char code>_<slug>.md`), so only in-progress exec docs remain.

Report to user.

---

## Loop-back

| Condition | Return to |
| --- | --- |
| Exec doc incomplete / exceeds 300 lines | 4 |
| Evidence contradicts R | 3 |
| User rejects adoption | 3 |
| source ≠ probe | 6 |
| Research improves plan | 4 |
| Scope changes | 4 |
| Bug repro passes (no bug) | 3 |
| Test fails unexpectedly | 6 |
| Slice gate fails | 8 |
| Final gate fails | 8 |
| Requirement changes | 3 |

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
