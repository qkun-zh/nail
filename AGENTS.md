# Agent Instructions

## Identity

You are a professional software engineer agent responsible for code implementation, debugging, and maintenance. Your work must follow this file (constitution) and `document/workflow.md` (execution loop). `README.md` is the human-facing project intro; it is not authoritative for engineering decisions.

## File System

- **Must read first** (in order):
  1. `AGENTS.md` — this file (constitution)
  2. `document/workflow.md` — execution loop
  3. `document/exec/` — if resuming, read the task's exec doc

- **Execution docs**: one per task, created at workflow §5, single source of truth during execution. Deleted when task complete.

- **Never touch**: `target/`, `dist/`, `data/`, `log/` (runtime/generated data)

## Architecture

Layering mandatory; dependencies point strictly inward.

**Backend** (`code/server/`):
```
interface → logic → repository
interface/logic/repository → infrastructure
```
- `main.rs` — composition root: config, logging, seed, start server.
- `interface` — axum routes: one `<verb>_<resource>` handler per route.
- `logic` — business rules, authorization, search, pagination.
- `repository` — agdb, SeekStorm, moka cache.
- `infrastructure` — config, logging, email, PDF, server bootstrap.

**Frontend** (`code/client/`):
```
router → page → request
page/request → infrastructure
```
- `main.rs` — composition root: runtime-config signals, mounts router.
- `router` — URL → `page` only.
- `page` — UI + local state; calls backend only via `request`.
- `request` — all HTTP, session tokens, `{code, data, message}` unwrap.
- `infrastructure` — wasm primitives (compile-time config, runtime config fetch, storage, PoW).

**Common** (`code/common/`):
- Shared data structures/methods (hash, PoW, name, tag, text, search, time, request/response).
- PoW lives inside common — no separate `pow` crate.
- Both depend on it; it depends on nothing internal — zero workspace-crate dependencies.

**Module org**: never `mod.rs`; module = same-named `.rs` + folder; deepen if dir > 16 files.

## Tech Stack

- **Frontend**: Leptos CSR via trunk.
- **Proxy**: pingap (static + reverse `/api/*`).
- **Backend**: axum, agdb, SeekStorm, moka, cedar-policy, lettre, tokio, tracing.
- **Hashing**: ascon; **IDs/tokens**: UUIDv7.
- **Versions**: pinned by each crate's `Cargo.lock`; pinned sources in `~/.cargo/registry/src/index.crates.io-*/`.

## Tools

### Allowed

- **Search**: prefer scoped roots (see Search Rules below). Repo-root `rg` needs `-g '!target' -g '!dist' -g '!data' -g '!log' -g '!*.lock' -g '!pingap-linux-gnu-x86-full'`
- **Edit**: Read/Edit/Write
- **Build/test**: `cargo fmt`, `cargo clippy` (zero warnings), `cargo test`; frontend also `trunk build`. Must use flags from `document/workflow.md`.

### Prohibited

- Hand-editing `Cargo.lock`
- `sed`/`awk`/`cat >` for editing files
- `unwrap`/`expect`/new panics
- Comments restating code

## Coding Standards

### Language and Naming

- English only — code, docs, comments, UI.
- No abbreviations (loop vars `i`/`j`/`k` excepted). Backend: CRUD-only verbs (`create`/`read`/`update`/`delete`) for every resource; collection reads are `read` (never `list`), paginated. Node ops, not flow vocabulary (no `intent=authenticate`).
- Config in toml, never hardcoded

### Size and Shape

- File ≤ 512 lines; function ≤ 256 lines.
- Concise; prefer pure functions; no hardcoding (toml); no dead code; zero-warning gate on every build.

### Comments

Only for non-obvious intent/constraints/tradeoffs; restating the code is a defect.

### Response Format

Every response: `{code, data, message}`. Errors propagate with `?`.

### Security

- Panic-free: never `unwrap`/`expect`.
- Errors: propagate with `?`; convert only at layer boundaries; interface maps to `{code, data, message}`.
- IDs/tokens: UUIDv7; hashing: ascon only.
- Authorization: enforced in `logic` against Cedar policies; every request goes through a principal session.
- Never write credentials to files, logs, diffs, commits
- Never commit secrets

## Configuration

Toml under `configuration/`, never hardcoded.

| File | Purpose | Notes |
| --- | --- | --- |
| `server.toml` | Runtime config | Read at startup |
| `front.toml` | Compile-time config | `include_str!`, fail fast |
| `email.toml` | Allowed domains | |

Backend serves `/config/read` for the frontend.

## Logging

`tracing` + `tracing-subscriber` to `log/`, daily pruning.

## Testing

- Test every function across all cases (exhaustive when cheap; else boundaries + randomized regular cases).
- Unit tests in `test/unit/{common,server,client}` via `#[path]`.
- Run `cargo test` in each crate; keep `cargo clippy` (zero warnings) and `cargo fmt` clean.
- Clippy runs plain (no `--all-targets`) so tests are exempt.

## Building & Running

- **Full-stack restart**: see `document/workflow.md` (Running the stack).
- **Backend alone**: `cargo run --bin server` (from `code/server`); seed samples with `-- seed-samples [count]`.
- **Frontend**: `trunk build` (from `code/client`), served by the proxy.

## Dependencies

Add one by one with `cargo add`, alphabetical, latest non-conflicting; commit `Cargo.lock`.

For any crate question: read the pinned source, then confirm with a probe test — source + probe evidence, never a guess. No implementation until both are recorded and the user adopts the plan (`document/workflow.md` §7).

## Search Rules

Search is precise, never the whole tree — most is generated/runtime data that pollutes results. Always set the search root; never search the repo root or a whole crate.

**Relevant roots** (search one of these, not above them):
- `code/{server,client,common}/src/` — Rust source.
- `test/`, `document/`, `configuration/` — those layers only.
- `code/{server,client,common}/Cargo.toml` — dependency declarations.

**Never include** (large or unrelated):
- `target/` — root and every crate (~14G combined).
- `code/client/dist/` — frontend build output.
- `code/proxy/pingap-linux-gnu-x86-full` — downloaded binary.
- `data/`, `log/`, `.git/`, `Cargo.lock`.

**Repo-root `rg`** (avoid when possible; prefer scoped roots above):
`rg -g '!target' -g '!dist' -g '!data' -g '!log' -g '!*.lock'
-g '!pingap-linux-gnu-x86-full'`. A `src/` root needs no exclusions.

## Version Control

- One commit per slice, clean tree
- Never discard work: no `git checkout --`/`git restore`/`reset --hard`/`clean -fd`
- Recover from commit or ask; discarding uncommitted changes needs approval each time

## Concurrent Agents

Other agents may work the same tree. Scope is generally disjoint.

- Don't be surprised by others' changes or commits
- Re-read files before depending on them
- Stage only your own work
- Never discard anyone's work (covers every change in the tree)
