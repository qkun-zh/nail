# Agent instructions

The project constitution is `README.md` — architecture layering, coding
standards, and build rules are mandatory, not advisory. This file tells agents
which engineering skills to reach for and which tool operations are allowed or
forbidden in this repo.

## Documentation map

- `README.md` — constitution (layering, standards, robustness, config, build).
- `document/handoff.md` — progress tracker: current state, what was done, what
  comes next. Update it at the end of every completed slice, before reporting.
- `document/agent-code-workflow.md` — the mandatory execution loop for writing
  code: baseline green, clean commits, requirement interrogation, todo plan,
  research via source/probe tests, red→green→gate→commit per slice, final gate.
- `document/adr/` — adjudicated decisions, numbered `0001-…`. Follow the
  numbering; do not reopen a frozen decision without a new ADR.
- `document/legacy/` — the original `nail` code. **Read-only reference**; never
  edit, delete, or depend on it for new work.
- `document/run.md` — full-stack build/restart procedure.

There is no issue tracker (no git remote, no `.scratch/`). Work is tracked in
`document/handoff.md`; specs/decisions live in `document/adr/`.

## When to use which skill

Invoke a skill (via the skill tool) when the current task matches its trigger.
Do not invoke a skill just to "have a vocabulary" — `codebase-design` is the
one exception below, and it is a reference to consult, not a session to run.

| Skill | Trigger — use it when … |
| --- | --- |
| `codebase-design` | designing or restructuring a module's interface, deciding where a seam goes, deepening a module, or making code more testable/AI-navigable. A vocabulary to consult during any design work, not a standalone session. |
| `diagnosing-bugs` | the user reports something broken, throwing, failing, or slow (or says "diagnose"/"debug"). Build a tight red-capable repro first; never hypothesise before it. |
| `tdd` | building a feature or fixing a bug test-first ("red-green-refactor", or tests were requested). Seams = the README §2 layer boundaries; tests live under `test/unit/{common,back,front}`, pulled in via `#[path]`. |
| `domain-modeling` | pinning down domain terminology, sharpening a fuzzy term, or recording an architectural decision (writes ADRs under `document/adr/`). |
| `grilling` | the user wants a plan/decision stress-tested ("grill me"). User-invoked; do not self-initiate. |
| `grill-with-docs` | a grilling session that should also produce glossary/ADR docs as it goes. User-invoked. |
| `improve-codebase-architecture` | the user wants a codebase architecture review / deepening-opportunities report. Produces an HTML report, then grills through the picked candidate. |
| `handoff` | wrapping up a session and compacting context for a fresh agent. Keep `document/handoff.md` current; the skill's temp-dir doc references it instead of duplicating it. |
| `thermo-nuclear-code-quality-review` | the user asks for an extremely strict maintainability / abstraction / spaghetti review. |
| `setup-matt-pocock-skills` | already run (this file carries its output). Do not re-run unless the user asks to reconfigure the issue tracker or doc layout. |
| `to-spec` | not usable as-is: it publishes to an issue tracker, which this repo does not have. Adapt to `document/handoff.md` or ask the user before running. |

## Tools and restrictions

### Allowed

- **Search**: Glob/Grep tools with `target/` and `dist/` excluded (add path
  limits or include filters like `*.rs`); for wide scans use `rg` with
  `-g '!target' -g '!dist'` in bash. The `document/legacy/` tree is 1.8 GB —
  never search it unless the task explicitly concerns legacy behavior.
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

- **Never edit `document/legacy/`** — read-only reference.
- **Never hand-edit `Cargo.lock`** — change dependencies only via `cargo add`
  (README §12); commit the lock.
- **Never touch runtime/generated data**: `target/`, `dist/`, `data/`, `log/`.
  `data/agdb` is reset and reseeded at startup; deleting the dir forces a
  fresh init. Do not commit any of these (see `.gitignore`).
- **Secrets**: never write `configuration/smtp.toml` contents or any
  credential into files, logs, diffs, or commits. Redact secrets in all
  output. The file is gitignored; the committed template is
  `configuration/smtp.toml.example`.
- **Per-slice commits are pre-authorized**: `document/agent-code-workflow.md`
  mandates one commit per slice on a clean tree, so each slice's commit needs no
  separate user prompt. `amend`, `push`, and `force` still require explicit user
  approval.
- **Never discard work**: `git checkout HEAD -- <path>`, `git checkout -- <path>`,
  `git restore`, `git reset --hard`, `git clean -fd`, and any `git stash` that
  drops changes are forbidden — they overwrite or delete uncommitted work with no
  recovery. To revert a file, recover it from a commit/bundle first, or ask the
  user. Discarding a file's uncommitted changes requires explicit user approval
  every time.
- **No `unwrap`/`expect`** and no new panics — README §5 is mandatory.
- **No comments restating the code** — README §4.5.

### Conventions to hold

- English only — code, docs, comments, UI strings (README §4.1).
- CRUD-only verbs for resource operations; node-op names, not frontend flow
  vocabulary (README §4.2).
- Every response is `{code, data, message}`; errors propagate with `?`
  (README §5, §7).
- Config lives in toml, never hardcoded (README §4.4).
