# Agent code workflow — the correctness loop

This document defines the mandatory execution loop an agent must run whenever
it writes or changes code in this repository. Its purpose: every change is
**accurate** (matches the user's real intent) and **correct** (green build,
zero warnings, passing tests, clean commits) before it is reported as done.

Constitution and standards remain `README.md`; the repo skill triggers and tool
restrictions remain `AGENTS.md`. This file only constrains the *order* of work.
Follow it top to bottom; never skip a phase, never advance past a failed phase
on a guess.

## The loop at a glance

```
while user provides a new requirement:
    execute_requirement(R)
```

```
procedure execute_requirement(R):

    # 1 — baseline: the tree must be green before we touch it
    assert_baseline(R)

    # 2 — git: start and end every slice on a clean tree
    clean_working_tree()

    # 3 — requirements: no assumptions, interrogate on doubt
    R = pin_down_requirement(R)

    # 4 — plan: decompose into independently verifiable slices
    todos = plan(R)

    # 5 — research: library source + probe tests, never guesses
    research(todos, R)                     # may send the loop back to step 3

    # 6 — execute: the inner red → green → verify → commit loop
    for todo in todos:
        test_first(todo)                   # red
        implement(todo)                    # green
        run_gate(todo)                     # fmt, clippy (zero warnings), tests
        commit_slice(todo)

    # 7 — final gate: reproduce the full green state, don't assert it
    final_gate()

    # 8 — handoff and report
    update_handoff()
    report_to_user()
```

Each phase below defines its exit condition. A phase completes only when its
condition holds; anything else returns the loop to the phase named under
"Loop-back rules".

---

## Phase 1 — `assert_baseline(R)`

```
procedure assert_baseline(R):
    if is_bugfix(R):                       # a broken tree IS the task
        write a failing test that reproduces the reported bug
        assert that test fails             # the repro is real, not imagined
        if it does not fail:               # the bug does not reproduce
            STOP and report                # leave the tree clean
            interrogate the user           # back to phase 3
        return
    run build:      must succeed
    run all tests:  must pass
    run clippy:     must be zero warnings
    run fmt:        must be clean
    if anything failed:
        STOP and report                     # never start from a broken tree
```

Rationale: you cannot attribute a regression to yourself if the tree was
already red. Exceptions exist only for the bug-fix task itself, and even then
only with a proven failing repro.

## Phase 2 — `clean_working_tree()`

```
procedure clean_working_tree():
    if working tree is dirty:
        commit the unrelated changes first, or stop and ask the user
    # invariant: work starts clean and every slice ends in a commit
```

Each slice of work is one commit; the tree is clean at every loop-back point.
Nothing stays uncommitted across phases.

## Phase 3 — `pin_down_requirement(R)`

```
procedure pin_down_requirement(R):
    loop:
        if R is precise and internally consistent:
            return R
        for each ambiguity or contradiction in R:
            interrogate the user directly      # a question, not a guess
```

The agent never invents behavior for a vague statement and never silently picks
one side of a contradiction. The user is the only authority on intent. The loop
exits only when `R` answers all of:

- exact behavior and observable outcome;
- scope: which files, layers, resources are affected;
- acceptance: how the user will verify the result.

## Phase 4 — `plan(R)`

```
procedure plan(R):
    todos = decompose(R)
    for todo in todos:
        assert todo is small, ordered, and independently verifiable
        assert todo has an exit test         # how do we know it is done?
    return todos
```

A todo is a slice, not a phase. Its exit test is the concrete, runnable
evidence that the slice is done — no "I think it works".

## Phase 5 — `research(todos, R)`: read the source, probe the behavior

**The most important phase in this document.** Almost every wrong line of code
an agent writes comes from assuming what a library or an existing module does
instead of reading what it actually does. Guessing API behavior is a defect,
not a shortcut.

```
procedure research(todos, R):
    for each unknown U required by todos:
        read the pinned library source:          # NEVER guess an API
            ~/.cargo/registry/src/index.crates.io-*/
        also read the repo's own module + tests  # it may already solve U
        if U is still ambiguous or untrustworthy:
            write a probe test and run it        # evidence over intuition
            record the verified behavior         # so the work is not re-done
        if the evidence contradicts R:
            interrogate the user                 # back to phase 3
```

### Read the pinned source — why

- The registry in `~/.cargo/registry/src/index.crates.io-*/` holds the exact
  versions pinned by `Cargo.lock`. Docs and blog posts describe other versions;
  only the pinned source describes this build.
- The repo's own layers are part of the same rule: before editing a module,
  read it and its tests. If the module already does what a todo needs, the todo
  changes — evidence over intention.
- When source is ambiguous, a probe test settles it. A probe test is a small,
  disposable `cargo test` that prints or asserts the actual behavior of one
  API. It is written to be wrong, run, and (usually) discarded — never shipped
  to satisfy a todo, never cited as evidence it has not produced.
- Record what the probe proved (a short note in `document/handoff.md`). This is
  what makes the loop replayable: the next agent does not re-derive the same
  facts.

### When a probe is mandatory

| Situation | Action |
| --- | --- |
| A third-party API's return type, error type, or side effects are not obvious from the signature | read source first, probe if still unclear |
| The pinned source contradicts what the agent believed | probe, then trust the probe |
| Two candidate calls look equivalent | probe to measure which actually meets the contract |
| An existing module's behavior is uncertain | read its code and tests; probe any remaining doubt |

Research covers not just third-party crates but also this repo's own layers.
Any mismatch between what the evidence shows and what the user stated is
reported back — never papered over. Research may also reveal that the plan from
phase 4 is misplaced or suboptimal; when it does, re-plan before executing.

## Phase 6 — execute: the inner loop

```
for todo in todos:
    write the failing test for todo          # RED
    assert it fails for the expected reason  # wrong failure = wrong understanding
    implement the minimal code to pass       # GREEN
    run_gate(todo):
        cargo fmt        must be clean
        cargo clippy     must be zero warnings
        cargo test       must pass (common, back, front)
    update document/handoff.md               # progress is kept current
    commit_slice(todo)                       # one commit per slice, clean message
```

The failing test is written first so the implementation is driven by an
observable contract, not by what "seems right". A test that fails for an
unexpected reason is a signal to go back to research, not to force it green.

## Phase 7 — `final_gate()`

```
procedure final_gate():
    full build          including frontend `trunk build` where applicable
    all test suites:    code/common, code/back, code/front
    clippy              zero warnings
    fmt                 clean
    if a test fails:
        re-run it once                     # distinguish flaky from real
        if it passes: flag the flakiness in the report
        else: fix, verify, commit, and re-run the gate
    # never report a red tree as done
```

The gate is reproduced, not assumed: the reported result must be obtainable by
running the same commands again.

## Phase 8 — `update_handoff()` + `report_to_user()`

```
procedure update_handoff():
    update document/handoff.md:
        what was done, what comes next, decisions taken
procedure report_to_user():
    summarize: slices completed, verification results, remaining risks
```

---

## Loop-back rules

| Condition | Go back to |
| --- | --- |
| Research/probe evidence contradicts the requirement | Phase 3 — interrogate, then re-plan |
| Research relocates or improves the plan (no contradiction) | Phase 4 — re-plan |
| Bug repro test passes (bug does not reproduce) | Phase 3 — report and interrogate |
| Test fails for an unexpected reason | Phase 5 — research the real cause |
| A slice's gate fails | Phase 6 — fix that slice |
| The requirement changes mid-work | Phase 3 — re-pin, re-plan |
| The final gate fails | Phase 6 — fix, re-verify |
| The user's answer changes scope | Phase 4 — re-decompose |

## Stop semantics

Any phase may terminate with `STOP and report` instead of looping back — for
example a bug that cannot be reproduced, a requirement that cannot be pinned
down (user unreachable), or a task that proves infeasible. A stop must:

- leave the working tree exactly as the report describes (committed slices stay
  committed; the state of any uncommitted work is called out explicitly);
- state the blocked phase and the reason;
- never proceed past the block on a guess.

## Invariants (never violated)

- No silent assumptions — every uncertainty is a question to the user.
- Library and API behavior is established by reading the pinned source and,
  when needed, a probe test — never by assumption (phase 5).
- No work on a broken tree except the bug-fix that reproduces it.
- One commit per slice; clean tree at every loop-back.
- Every reported result is reproducible by re-running the gates.
- Repo constitution always applies: no hand-edited `Cargo.lock`, no
  `unwrap`/`expect`, no edits to `document/legacy/`, no secrets in output.
- Never discard work: `git checkout HEAD -- <path>`, `git checkout --`,
  `git restore`, `git reset --hard`, `git clean -fd`, and change-dropping
  `git stash` are forbidden (see AGENTS.md). Recover from a commit/bundle or
  ask the user before reverting any uncommitted change.
