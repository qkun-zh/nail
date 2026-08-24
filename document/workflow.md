# Workflow

**Purpose:** Evidence-based code changes with mandatory feedback loops.

**Notation:** `R` = requirement, `gate` = binary pass/fail checkpoint.

---

## Execution loop

```
execute(R):
  1  baseline(R)              §1
  2  clean-tree               §2
  3  propose(R)               §3
  4  research(R)              §4
  5  resolve(R, research)     §5
  6  plan(R, research)        §6
  7  exec-doc(R, plan)        §7
  8  gate-adopt               §8
  9  for-slice(plan)          §9
 10  gate-final               §10
```

---

## §1 baseline

**Input:** `R` | **Output:** green tree

```
cargo test -j 1 -p {server|common|emailer|client}
```

- One crate per invocation, `-j 1` mandatory.
- Bugfix: failing repro test first; passes → STOP.
- Non-bugfix: fail → STOP.

---

## §2 clean-tree

**Input:** working tree | **Output:** clean tree

Uncommitted unrelated changes → commit or ask. Never discard.

---

## §3 propose

**Input:** `R` (raw) | **Output:** `R₀`

Write one precise, testable requirement. Ambiguity → ask. `R₀` provisional.

---

## §4 research

**Input:** `R₀` | **Output:** research report

**File:** `document/research/<4-char>_<slug>.md` (≤300 lines; lifecycle §10)

**Update:** after each unknown resolved (source + probe complete).

**Sections:**
1. Requirement — `R₀`
2. Research questions — unknowns list
3. Evidence — per unknown: source (code read) + probe (test)
4. Findings — discoveries
5. Impact on R — revision needed? New `R₁`?
6. Open items — user input needed

**Evidence rules:**
- Every unknown: source + probe, `source ≠ probe`.
- Contradicts `R` → update report, goto §3.

**Verification dimensions:**

| Dimension | Check |
|-----------|-------|
| Correctness | matches `R`, normal + edge |
| Behavior change | I/O delta vs baseline = `R` |
| Complexity | Big-O, allocations |
| Performance | latency/throughput delta |

**Probe:** `test/unit/{area}/probe_<NNN>_<purpose>.rs`, 3-digit zero-padded.
After slice gate: promote or delete.

---

## §5 resolve

**Input:** research report, `R₀` | **Output:** `R`

```
if findings contradict R₀:
  R = revise(R₀)  # in-place update
  update research report
else:
  R = R₀
goto §6
```

---

## §6 plan

**Input:** `R`, research report | **Output:** slice list

Each slice: **Goal** (1 sentence), **Files** (exact), **Red** (test fails), **Green** (behavior), **Exit test** (command).

Unknowns → flag in exec doc, don't block.

---

## §7 exec-doc

**Input:** `R`, plan | **Output:** exec doc

**File:** `document/exec/<4-char>_<slug>.md` (≤300 lines; lifecycle §10)

**Update:** after each slice complete.

**Sections:**
1. Requirement — `R`, acceptance criteria
2. Scope — in/out
3. Design decisions — modules, trade-offs
4. Slice breakdown — from §6
5. Open unknowns — evidence source
6. Verification plan — per slice
7. Risks — failure modes
8. Constraints — prohibitions
9. Questions — for user

---

## §8 gate-adopt

**Input:** research report, exec doc | **Output:** adoption

Gate passes iff:
1. Evidence consistent across dimensions.
2. User adopts plan.

Rejection → §3.

---

## §9 slice loop

**Input:** exec doc | **Output:** commit + CI green

Per slice:
```
red:    test → smoke-pass → must fail
green:  impl → smoke-pass → must pass
gate:   fmt + clippy → clean
commit: one commit, clean tree
push:   git push → CI gate
handoff: exec doc + handoff doc sync
```

**CI gate:** GitHub Actions (fmt, clippy, tests, wasm, audit).
- `git push` exits 0 AND `git log origin/main..HEAD` empty.
- Watch: `document/ci-watch.sh --once` or `bg [timeout]`.
- Fail → debug, fix, re-gate.

**Resource contention:** check load before local build. Never on busy machine.

---

## §10 gate-final

**Input:** all slices committed, CI green | **Output:** task complete

Full CI pass. Fail → §9.

Cleanup: delete research report + exec doc + handoff task file.

**Unified artifact lifecycle:** the three per-task artifacts — research
report (`document/research/`), exec doc (`document/exec/`) and handoff task
file (`document/handoff/`) — share one naming scheme, `<4-char code>_<slug>.md`,
and one cleanup rule: each is deleted automatically as soon as this gate
completes with a green CI run. Keep none past gate-final.

---

## Handoff

**Mandatory after each slice.** Sync exec doc + handoff doc.

`document/handoff/readme.md`: state, slices done, decisions, risks. Stale = gate fail.

---

## Loop-back

| Condition | Return |
|-----------|--------|
| Exec doc incomplete | §7 |
| User rejects | §3 |
| source ≠ probe | §4 |
| Research improves plan | §6 |
| Scope changes | §6 |
| Bug repro passes | §3 |
| Test fails | §4 |
| Slice gate fails | §9 |
| Final gate fails | §10 |
| Requirement changes | §3 |

---

## STOP

At any phase: stop, tree as-is, report block. Never guess.

---

## Invariants

1. Uncertainty → question → answer, never guess.
2. No code without research report + exec doc + adoption.
3. Official APIs win; custom wheel needs consent.
4. Dimensions evidenced or N/A before gate.
5. One commit per slice; clean tree.
6. Reproducible gates.
7. No `Cargo.lock` edit, no `unwrap`, no secrets.
8. Never discard work.
9. Docs update at phase boundary; verify after update.
