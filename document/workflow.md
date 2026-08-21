# Workflow — the correctness loop

Mandatory for every code change. Authority: `README.md`, `AGENTS.md`; this
file wins on execution order.

## Environment

- Roots: `WORKSPACE=/home/qkun/nail/code`, `FRONT=/home/qkun/nail/code/front`,
  `PROXY=/home/qkun/nail/code/proxy/pingap-linux-gnu-x86-full`,
  `CFG=/home/qkun/nail/configuration/proxy`.
- Toolchain: stable (same as CI). No extra flags, no `--release` (official
  LLVM builds only).

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

Local `cargo test` is a smoke pass only; the test gate is always CI (§8).

---

## §1 Baseline

- Smoke pass green → proceed. One crate per invocation, `-j 1` mandatory
  (parallel crate builds exhaust RAM), crates never combined, from
  `WORKSPACE`: `cargo test -j 1 -p {nail_back|nail_common|emailer|nail_front}`
  (`nail_front` = host tests); pow: `cargo test -j 1 -p pow --all-targets`.
- Bug fix → failing repro test first; it must fail before anything else.
- Red and not a bug fix → STOP; fix the tree first.

## §2 Clean tree

Uncommitted changes unrelated to R → commit or ask. Never discard.

## §3 Pin requirements

R must be unambiguous and internally consistent; any ambiguity → ask.
Outcome: one precise, testable requirement statement.

## §4 Plan

Ordered slices, each stating: **Goal** (one sentence), **Files** (exact),
**Red** (test that must fail first), **Green** (expected behavior),
**Exit test** (command proving completion). Unknowns → §6: flag, don't block.

## §5 Exec doc

`document/exec/<4-char code>_<slug>.md` — random alphanumeric code, unique,
never reused, invented by the agent (no scripts/system calls). Under 300
lines. Deleted at completion (§9). Single source of truth during execution:
read at every slice start, updated in place when evidence contradicts, with
a trailing `## Change log`.

Required sections (empty only as explicit "N/A" + one-line reason):

1. **Requirement** — pinned R, acceptance criteria
2. **Scope** — in-scope, explicitly out-of-scope
3. **Design decisions** — modules, seams, trade-offs, rationale
4. **Slice breakdown** — from §4, one entry per slice
5. **Open unknowns** — evidence source per item (source/probe)
6. **Verification plan** — dimensions per slice, how verified
7. **Risks** — failure modes, mitigation, rollback
8. **Constraints** — task-specific prohibitions
9. **Questions** — unresolved ambiguities for the user

## §6 Evidence

Every unknown gets **source** (pinned code read) + **probe** (disposable
test). `source ≠ probe` → resolve before proceeding; contradicts R → ask.

Evidence is mandatory when: return/side effects are unclear; source
contradicts belief (probe wins); two APIs look equivalent (probe decides).
Behavior visible in source needs source only.

Verification dimensions — each applicable row `verified` or `N/A` + reason;
`unknown` → back to research; an unevidenced dimension blocks the gate.

| Dimension | Check |
| --- | --- |
| Correctness | matches R, normal + edge cases |
| Behavior change | input/output delta vs baseline = R |
| Time / space complexity | Big-O; allocations, DB footprint |
| Performance | latency/throughput delta vs baseline |

Reuse before build: official APIs first; a custom wheel or workaround needs
user consent plus recorded rejection reasons.

### Probes

One file per probe, never a shared `probe.rs`, at
`test/unit/{common,back,front}/<area>/probe_<NNN>_<purpose>.rs`. `<NNN>`:
3-digit zero-padded, repo-unique, lowest first. First line: doc comment with
purpose, source evidence, acceptance question. Function: `probe_<NNN>_<purpose>`.
One agent per number; after the gate, promote (rename, move) or delete.

## §7 Gate (adoption)

No code before: (1) evidence consistent across applicable dimensions,
(2) user explicitly adopts the plan. Rejection → §3.

## §8 Slice loop

```
red:    write test → smoke pass (§1) → must fail
green:  implement → smoke pass → must pass
gate:   cargo fmt --check && cargo clippy -D warnings → clean
commit: one commit per slice, clean tree; docs-only → `[skip ci]` prefix
push:   git push origin main → CI gate
```

CI gate — GitHub Actions, never the local machine. Push runs
`.github/workflows/ci.yml`: fmt, clippy, tests (pow, common, back, front
host), wasm build, security audit.

- Pushed ⇔ `git push` exits 0 and `git log origin/main..HEAD` is empty.
- Watch via `document/ci-watch.sh`: `--once` (single check) or `bg [timeout]`
  (background, logs `/tmp/ci-watch.log`, poll with `tail -f`).
- Failing job = failed gate → debug, fix, re-gate. Never skip a gate.

Resource contention: prefer CI; before any local build/test (here, §9, or
probes) check load (`uptime`, `ps -eo pcpu`, `mpstat`); loaded → back off and
poll. Never build on a busy machine — a shared tree gives unreliable results.

## §9 Handoff

Mandatory before reporting done: the next session must need no memory of this
one. Update `document/handoff/readme.md` and the per-task file: current
state, slices done, decisions, remaining risks. Drop completed slices; label
ownership; never touch others' tasks; use the 64-em-dash divider. Stale or
incomplete handoff = red gate. Task fully complete → delete its exec doc.
Report to the user.

## §10 Final gate

Full build + all tests + clippy (0 warnings) + fmt, gated by CI: push `main`
and confirm `document/ci-watch.sh` reports success. Must reproduce green;
never report red. Fail → §8.

---

## Running the stack

Full-stack restart, for debugging or manual verification:

1. Frontend: `env -u NO_COLOR trunk build` (from `FRONT`)
2. Backend: `cargo run -p nail_back` (from `WORKSPACE`); background:
   `setsid nohup cargo run -p nail_back > /home/qkun/nail/log/back/run.log 2>&1 < /dev/null &`
3. Proxy: `PROXY -c CFG`; background:
   `setsid nohup PROXY -c CFG > /home/qkun/nail/log/proxy/run.log 2>&1 < /dev/null &`

Health checks: backend `curl -sf http://127.0.0.1:3000/config/read`; via
proxy `curl -sf http://127.0.0.1:8080/api/config/read`.

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

At any phase: stop, leave the tree as-is, report blocked phase + reason.
Never proceed past a block on a guess.

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
