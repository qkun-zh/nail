# Agent code workflow — the correctness loop

Mandatory order for every code change. Standards: `README.md`; skills and tool
rules: `AGENTS.md`.

## Loop

```
execute(R):
  1  assert baseline green            # bugfix: failing repro test first, else STOP
  2  clean working tree               # dirty → commit unrelated or ask
  3  pin_down(R)                      # precise & consistent, else ask each ambiguity
  4  todos = plan(R)                  # small, ordered, verifiable, exit test, unknowns
  5  evidence = research(todos)       # source + probe double evidence
  5.5 evidence_gate(evidence)         # explicit adoption before any code
  6  for todo in todos: red → green → gate → commit
  7  final_gate()                     # reproduce green; never report red
  8  update_handoff(); report()
```

## Phase 5 — evidence (mandatory)

Never guess. Every unknown gets `source` (pinned lib/repo module read) + `probe`
(disposable test; promote if it proves a constraint, surprise, or boundary).
`source ≠ probe` → resolve; contradicts R → ask.

**Evidence mandatory when**

| Situation | Evidence |
| --- | --- |
| Return/error type or side effects unclear | source first; probe mandatory |
| Source contradicts belief | probe, then trust probe |
| Two calls look equivalent | probe to pick correct |
| Repo module behavior uncertain | read + tests; probe remaining doubt |
| Behavior directly observable in source | source suffices |

**Verification dimensions** — "tests pass" alone is not evidence. Each applicable
dimension must be `verified` or `N/A + one-line reason`; `unknown` routes back to
phase 5. An unevidenced applicable dimension blocks the gate.

| Dimension | Verify | Evidence |
| --- | --- | --- |
| Correctness | behavior matches R, normal + edge cases | tests, probe |
| Behavior change | input/output delta vs baseline = R | before/after probes, diffs |
| Time complexity | Big-O of touched path | source + probe/benchmark |
| Space complexity | allocations, cache, DB footprint | source + probe |
| Performance change | latency/throughput delta vs baseline | benchmark hot paths |

**Reuse before build** — never reinvent. If a standard/pinned official API
covers the need, use it. A custom wheel or patch-style code (workaround, shim,
adapter masking a defect/gap) requires explicit user consent before any
implementation. Record searched APIs and rejection reasons, or the consent;
"didn't look" is not a verdict.

**Adoption gate (5.5)** — no code until evidence is consistent per dimension and
the user explicitly adopts the plan.

**Probe file layout (concurrent-safe)** — never accumulate probes into a single
shared `probe.rs`. Concurrent agents editing one file collide; one file per
probe avoids that. Rules:

- Each probe lives in its **own** file under
  `test/{common,back,front}/<area>/probe_<NNN>_<purpose>.rs`, wired into the
  suite by its own `#[path]` module declaration in the area harness
  (e.g. `test/unit/back/harness.rs`).
- `<NNN>` is a 3-digit zero-padded sequence unique across the whole repo,
  allocated lowest-first (add to a single counter). `<purpose>` is a short
  snake_case phrase naming what the probe verifies.
- First line of the file is a doc comment: numbered purpose, the source
  evidence it confirms, and the acceptance question it answers.
- Test fn mirrors the file: `probe_<NNN>_<purpose>`.
- One agent claims one number; do not edit another agent's `probe_<NNN>_*` file.
- Promote-to-real: rename file and fn (drop `NNN_`), move under the normal test
  name; disposable probes are deleted after the evidence gate.

## Slices (6–8)

- Each slice: test first (red) → implement (green) → gate (fmt, clippy 0 warn,
  tests pass) → one commit, clean tree.
- Final gate: build + all tests + clippy + fmt; never report red.
- Handoff: record done/next/decisions; report slices, verification, remaining
  risks.

## Loop-back

| Condition | To |
| --- | --- |
| Evidence contradicts R / adoption not accepted | 3 |
| Source ≠ probe | 5 |
| Research improves plan / scope changes | 4 |
| Bug repro passes (no bug) | 3 |
| Test fails unexpectedly | 5 |
| Slice/final gate fails | 6 |
| Requirement changes | 3 |

## Stop

May `STOP and report`: leave the tree as described, state blocked phase + reason;
never proceed past a block on a guess.

## Invariants

- Every uncertainty is a question, never a guess.
- No code without double evidence + explicit adoption (5.5).
- API behavior from source + probe; official APIs win; wheel/patch needs consent.
- Verification covers correctness, behavior change, time & space complexity, and
  performance — evidenced or justified N/A before any gate.
- No work on a broken tree (except reproducing a bug); one commit per slice;
  clean at every loop-back; every result reproducible by re-running gates.
- No hand-edited `Cargo.lock`; no `unwrap`/`expect`; no secrets in output.
- Never discard work (`checkout --`/`restore`/`reset --hard`/`clean -fd`/
  drop-stash); recover from a commit or ask.
