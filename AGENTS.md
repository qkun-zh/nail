# Agent instructions

## Read these first, in order, before any work

1. `README.md` — the project constitution: repository layout, architecture
   layering, coding standards, robustness, configuration, and build rules.
   Mandatory, not advisory.
2. `document/INDEX.md` — the doc map and read order.
3. `document/workflow.md` — the mandatory execution loop for any code change:
   baseline green, clean commits, requirement interrogation, todo plan, research
   via source/probe tests, red→green→gate→commit per slice, final gate.

Everything below — the skills to reach for and the tool operations allowed or
forbidden — is secondary to those documents. If this file ever conflicts with
any of them, the README, the doc index, and the workflow doc win.

## Concurrent agents

Other agents may work on this same project at the same time. Task scopes are
generally disjoint, so:

- Do not be surprised by uncommitted modifications or new commits you did not
  make — someone else is working in the same tree.
- Do not assume files you did not touch are in the state you left them; re-read
  before depending on them.
- Do not sweep unrelated working-tree changes into your own commit — stage only
  what belongs to your task, and leave other agents' in-progress work alone.
- Never discard others' work: the "never discard work" rule applies to every
  change in the tree, not just your own.

## Documentation map

- `README.md` — constitution (layering, standards, robustness, config, build).
- `document/INDEX.md` — entry point: read order and what each doc covers.
- `document/workflow.md` — the mandatory code-execution loop.
- `document/handoff.md` — progress tracker: current state, what was done, what
  comes next. Update it at the end of every completed slice, before reporting.
- `document/decisions.md` — the decided architecture and conventions. Read-only;
  changing one requires re-evaluation.
- `document/run.md` — full-stack build/restart procedure and health checks.

There is no issue tracker (no git remote, no `.scratch/`). Work is tracked in
`document/handoff.md`; specs/decisions live in `document/decisions.md`.

## When to use which skill

Invoke a skill (via the skill tool) when the current task matches its trigger.
Do not invoke a skill just to "have a vocabulary" — `codebase-design` is the
one exception below, and it is a reference to consult, not a session to run.

| Skill | Trigger — use it when … |
| --- | --- |
| `codebase-design` | designing or restructuring a module's interface, deciding where a seam goes, deepening a module, or making code more testable/AI-navigable. A vocabulary to consult during any design work, not a standalone session. |
| `diagnosing-bugs` | the user reports something broken, throwing, failing, or slow (or says "diagnose"/"debug"). Build a tight red-capable repro first; never hypothesise before it. |
| `tdd` | building a feature or fixing a bug test-first ("red-green-refactor", or tests were requested). Seams = the README layer boundaries; tests live under `test/unit/{common,back,front}`, pulled in via `#[path]`. |
| `domain-modeling` | pinning down domain terminology, sharpening a fuzzy term, or recording an architectural decision (writes to `document/decisions.md`). |
| `grilling` | the user wants a plan/decision stress-tested ("grill me"). User-invoked; do not self-initiate. |
| `grill-with-docs` | a grilling session that should also produce glossary/decision docs as it goes. User-invoked. |
| `improve-codebase-architecture` | the user wants a codebase architecture review / deepening-opportunities report. Produces an HTML report, then grills through the picked candidate. |
| `handoff` | wrapping up a session and compacting context for a fresh agent. Keep `document/handoff.md` current; the skill's temp-dir doc references it instead of duplicating it. |
| `thermo-nuclear-code-quality-review` | the user asks for an extremely strict maintainability / abstraction / spaghetti review. |
| `setup-matt-pocock-skills` | already run (this file carries its output). Do not re-run unless the user asks to reconfigure the issue tracker or doc layout. |
| `to-spec` | not usable as-is: it publishes to an issue tracker, which this repo does not have. Adapt to `document/handoff.md` or ask the user before running. |

## Tools and restrictions

### Allowed

- **Search**: Glob/Grep tools with `target/` and `dist/` excluded (add path
  limits or include filters like `*.rs`); for wide scans use `rg` with
  `-g '!target' -g '!dist'` in bash.
- **File edits**: Read/Edit/Write tools. Bash is for build, test, git, and
  infra commands — not for file manipulation (`sed`/`awk`/`cat >` are
  forbidden for editing).
- **Skills**: via the skill tool, on the triggers above.
- **Crate questions**: read the pinned crate source in
  `~/.cargo/registry/src/index.crates.io-*/` first; if ambiguous, write a
  probe test (`cargo test`) rather than guessing.
- **Verification**: `cargo fmt`, `cargo clippy` (zero-warning gate), and
  `cargo test` inside `code/{common,back,front}`. Frontend changes also need
  a `trunk build` (see `document/run.md`).

### Prohibited

- **Never hand-edit `Cargo.lock`** — change dependencies only via `cargo add`
  (README §Dependencies); commit the lock.
- **Never touch runtime/generated data**: `target/`, `dist/`, `data/`, `log/`.
  `data/agdb` is reset and reseeded at startup; deleting the dir forces a
  fresh init. Do not commit any of these (see `.gitignore`).
- **Secrets**: never write `configuration/smtp.toml` contents or any
  credential into files, logs, diffs, or commits. Redact secrets in all
  output. The file is gitignored; the committed template is
  `configuration/smtp.toml.example`.
- **Per-slice commits are pre-authorized**: `document/workflow.md`
  mandates one commit per slice on a clean tree, so each slice's commit needs no
  separate user prompt. `amend`, `push`, and `force` still require explicit user
  approval.
- **Never discard work**: `git checkout HEAD -- <path>`, `git checkout -- <path>`,
  `git restore`, `git reset --hard`, `git clean -fd`, and any `git stash` that
  drops changes are forbidden — they overwrite or delete uncommitted work with no
  recovery. To revert a file, recover it from a commit/bundle first, or ask the
  user. Discarding a file's uncommitted changes requires explicit user approval
  every time.
- **No `unwrap`/`expect`** and no new panics — README §Robustness is mandatory.
- **No comments restating the code** — README §Comments.

### Conventions to hold

- English only — code, docs, comments, UI strings (README §Language).
- CRUD-only verbs for resource operations; node-op names, not frontend flow
  vocabulary (README §Naming).
- Every response is `{code, data, message}`; errors propagate with `?`
  (README §Robustness, §Backend rules).
- Config lives in toml, never hardcoded (README §Configuration).