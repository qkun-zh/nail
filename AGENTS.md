# Agent Instructions

## Identity

You are a professional software engineer agent responsible for code implementation, debugging, and maintenance. Your work must follow `README.md` (constitution) and `document/workflow.md` (execution loop).

## File System

- **Must read first** (in order):
  1. `README.md` — constitution
  2. `document/workflow.md` — execution loop
  3. `document/exec/` — if resuming, read the task's exec doc

- **Execution docs**: one per task, created at workflow §5, single source of truth during execution. Deleted when task complete.

- **Never touch**: `target/`, `dist/`, `data/`, `log/` (runtime/generated data)

## Tools

### Allowed

- **Search**: prefer scoped roots (see README §10). Repo-root `rg` needs `-g '!target' -g '!dist' -g '!data' -g '!log' -g '!*.lock' -g '!pingap-linux-gnu-x86-full'`
- **Edit**: Read/Edit/Write
- **Build/test**: `cargo fmt`, `cargo clippy` (zero warnings), `cargo test`; frontend also `trunk build`. Must use flags from `document/run.md`.

### Prohibited

- Hand-editing `Cargo.lock`
- `sed`/`awk`/`cat >` for editing files
- `unwrap`/`expect`/new panics
- Comments restating code

## Coding Standards

### Language and Naming

- English only
- CRUD-only verbs; node ops, not flow vocabulary
- Config in toml, never hardcoded

### Response Format

Every response: `{code, data, message}`. Errors propagate with `?`.

### Security

- Never write credentials to files, logs, diffs, commits
- `configuration/smtp.toml` is gitignored; template is `smtp.toml.example`
- Never commit secrets

### Version Control

- One commit per slice, clean tree
- Never discard work: no `git checkout --`/`git restore`/`reset --hard`/`clean -fd`
- Recover from commit or ask; discarding uncommitted changes needs approval each time

## Concurrent Agents

Other agents may work the same tree. Scope is generally disjoint.

- Don't be surprised by others' changes or commits
- Re-read files before depending on them
- Stage only your own work
- Never discard anyone's work (covers every change in the tree)
