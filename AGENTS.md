# Agent instructions

## Read first, in order, before any work

1. `README.md` — constitution: layout, layering, standards, robustness, config,
   build. Mandatory.
2. `document/workflow.md` — mandatory loop: baseline → clean → pin → plan →
   exec doc → evidence → adoption → slice loop → final gate → handoff.
3. `document/exec/` — if resuming, read the task's exec doc first (single
   source of truth for context, decisions, and plan).

These plus this file cross-reference and win over everything below.

## Concurrent agents

Other agents may work the same tree. Scope is generally disjoint: don't be
surprised by others' changes or commits; re-read files before depending on
them; stage only your own work; never discard anyone's work (the "never
discard" rule covers every change in the tree).

## Documentation map

- `README.md` — constitution.
- `document/workflow.md` — mandatory execution loop.
- `document/exec/` — execution documents: one per task, numbered `NNN_slug.md`.
  Written at workflow §5 before any code. Single source of truth during work.
- `document/handoff.md` — progress: current state, done, next. Update at the end
  of every completed slice, before reporting.
- `document/run.md` — build/restart/health-check.

No issue tracker (no git remote, no `.scratch/`). Work tracked in
`document/handoff.md`.

## When to use which skill

Invoke a skill when the task matches its trigger; don't invoke for vocabulary
only (`codebase-design` is the one reference-only exception).

| Skill | Trigger |
| --- | --- |
| `codebase-design` | designing/restructuring a module interface, choosing a seam, deepening a module, AI-navigability. Reference vocabulary, not a session. |
| `diagnosing-bugs` | something broken/throwing/failing/slow ("diagnose"/"debug"). Tight red-capable repro first; never hypothesise before it. |
| `tdd` | building/fixing test-first. Seams = README layer boundaries; tests in `test/unit/{common,back,front}` via `#[path]`. |
| `domain-modeling` | pinning domain terms or recording a decision. |
| `grilling` | user wants a plan/decision stress-tested ("grill me"). User-invoked. |
| `grill-with-docs` | grilling that also produces glossary/decision docs. User-invoked. |
| `improve-codebase-architecture` | architecture review/deepening report (HTML) then grill. |
| `handoff` | wrapping up; keep `document/handoff.md` current. |
| `thermo-nuclear-code-quality-review` | extremely strict maintainability/abstraction/spaghetti review. |
| `setup-matt-pocock-skills` | already run; don't re-run unless reconfigure requested. |
| `to-spec` | not usable as-is (no issue tracker); adapt to `document/handoff.md` or ask. |

## Tools

### Allowed

- **Search**: precise scope only — set the root, never search the repo root or
  a whole crate; see `README.md` §14 for roots and exclusions. Repo-root `rg`
  needs `-g '!target' -g '!dist' -g '!data' -g '!log' -g '!*.lock'
  -g '!pingap-linux-gnu-x86-full'`; a `src/` root needs none.
- **Edits**: Read/Edit/Write. Bash only for build/test/git/infra, not file
  manipulation (`sed`/`awk`/`cat >` forbidden for editing).
- **Skills**: via skill tool, on the triggers above.
- **Crate questions**: read pinned source in
  `~/.cargo/registry/src/index.crates.io-*/` first, then verify with a probe
  test (`cargo test`) — source + probe evidence. No implementation until both
   are recorded and the user adopts the plan (`workflow.md` §7).
- **Verify**: `cargo fmt`, `cargo clippy` (zero warnings), `cargo test` in
  `code/{common,back,front}`; frontend also `trunk build` (`document/run.md`).

### Prohibited

- Never hand-edit `Cargo.lock` — change deps only via `cargo add`; commit the
  lock.
- Never touch runtime/generated data: `target/`, `dist/`, `data/`, `log/`.
  `data/agdb` resets/reseeds at startup; don't commit these.
- **Secrets**: never write `configuration/smtp.toml` or credentials into files,
  logs, diffs, commits; redact in all output. Gitignored; template is
  `smtp.toml.example`.
- **Per-slice commits pre-authorized**: one commit per slice on a clean tree
  needs no prompt. `amend`, `push`, `force` need explicit approval.
- **Never discard work**: no `git checkout --`/`git restore`/`reset --hard`/
  `clean -fd`/change-dropping `stash`. Recover from a commit/bundle or ask;
  discarding uncommitted changes needs explicit approval each time.
- No `unwrap`/`expect`/new panics (README §Robustness).
- No comments restating the code (README §Comments).

### Conventions to hold

- English only (README §Language).
- CRUD-only verbs; node ops, not flow vocabulary (README §Naming).
- Every response `{code, data, message}`; errors propagate with `?` (README
  §Robustness/§Backend).
- Config in toml, never hardcoded (README §Configuration).