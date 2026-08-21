# nail

Versioned-article knowledge base: authors publish versioned revisions, attach
notes/comments, tag, and search. Access via email-challenge auth with PoW,
Cedar authorization, PDF download with short-lived tokens.

**This file is the constitution.** Read it, `AGENTS.md`, and `document/workflow.md`
before any work — they cross-reference and must be read together.

## 1. Architecture

Layering mandatory; dependencies point strictly inward.

**Backend** (`code/back/`):
```
interface → logic → repository
interface/logic/repository → infrastructure
```
- `main.rs` — composition root: config, logging, seed, start server.
- `interface` — axum routes: one `<verb>_<resource>` handler per route.
- `logic` — business rules, authorization, search, pagination.
- `repository` — agdb, SeekStorm, moka cache.
- `infrastructure` — config, logging, email, PDF, server bootstrap.

**Frontend** (`code/front/`):
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
- Both depend on it; it depends on nothing internal.

**Module org**: never `mod.rs`; module = same-named `.rs` + folder; deepen if dir > 16 files.

## 2. Tech Stack

- **Frontend**: Leptos CSR via trunk.
- **Proxy**: pingap (static + reverse `/api/*`).
- **Backend**: axum, agdb, SeekStorm, moka, cedar-policy, lettre, tokio, tracing.
- **Hashing**: ascon; **IDs/tokens**: UUIDv7.
- **Versions**: pinned by each crate's `Cargo.lock`; pinned sources in `~/.cargo/registry/src/index.crates.io-*/`.

## 3. Coding Standards

- **Language**: English only — code, docs, comments, UI.
- **Naming**: no abbreviations (loop vars `i`/`j`/`k` excepted). Backend: CRUD-only verbs (`create`/`read`/`update`/`delete`) for every resource; collection reads are `read` (never `list`), paginated. Node ops, not flow vocabulary (no `intent=authenticate`).
- **Size**: file ≤ 512 lines; function ≤ 256 lines.
- **General**: concise; prefer pure functions; no hardcoding (toml); no dead code; zero-warning gate on every build.
- **Comments**: only for non-obvious intent/constraints/tradeoffs; restating the code is a defect.

## 4. Robustness & Security

- **Panic-free**: never `unwrap`/`expect`.
- **Errors**: propagate with `?`; convert only at layer boundaries; interface maps to `{code, data, message}`.
- **IDs/tokens**: UUIDv7; **hashing**: ascon only.
- **Authorization**: enforced in `logic` against Cedar policies; every request goes through a principal session.

## 5. Configuration

Toml under `configuration/`, never hardcoded.

| File | Purpose | Notes |
| --- | --- | --- |
| `server.toml` | Runtime config | Read at startup |
| `front.toml` | Compile-time config | `include_str!`, fail fast |
| `email.toml` | Allowed domains | |
| `smtp.toml` | Secrets | Gitignored; template `smtp.toml.example` |

Backend serves `/config/read` for the frontend.

## 6. Response Format

Every response: `{code, data, message}` (code=status, message=reason, data=payload).
Logging: `tracing` + `tracing-subscriber` to `log/`, daily pruning.

## 7. Testing

- Test every function across all cases (exhaustive when cheap; else boundaries + randomized regular cases).
- Unit tests in `test/unit/{common,back,front}` via `#[path]`.
- Run `cargo test` in each crate; keep `cargo clippy` (zero warnings) and `cargo fmt` clean.
- Clippy runs plain (no `--all-targets`) so tests are exempt.

## 8. Building & Running

- **Full-stack restart**: see `document/workflow.md` (Running the stack).
- **Backend alone**: `cargo run --bin nail_back` (from `code/back`); seed samples with `-- seed-samples [count]`.
- **Frontend**: `trunk build` (from `code/front`), served by the proxy.

## 9. Dependencies

Add one by one with `cargo add`, alphabetical, latest non-conflicting; commit `Cargo.lock`.

For any crate question: read the pinned source, then confirm with a probe test — source + probe evidence, never a guess. No implementation until both are recorded and the user adopts the plan (`document/workflow.md` §7).

## 10. Search Rules

Search is precise, never the whole tree — most is generated/runtime data that pollutes results. Always set the search root; never search the repo root or a whole crate.

**Relevant roots** (search one of these, not above them):
- `code/{back,front,common}/src/` — Rust source.
- `test/`, `document/`, `configuration/` — those layers only.
- `code/{back,front,common}/Cargo.toml` — dependency declarations.

**Never include** (large or unrelated):
- `target/` — root and every crate (~14G combined).
- `code/front/dist/` — frontend build output.
- `code/proxy/pingap-linux-gnu-x86-full` — downloaded binary.
- `data/`, `log/`, `.git/`, `Cargo.lock`.

**Repo-root `rg`** (avoid when possible; prefer scoped roots above):
`rg -g '!target' -g '!dist' -g '!data' -g '!log' -g '!*.lock'
-g '!pingap-linux-gnu-x86-full'`. A `src/` root needs no exclusions.
