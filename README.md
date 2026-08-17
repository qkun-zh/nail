# nail

Versioned-article knowledge base: authors publish versioned revisions, attach
notes/comments, tag, and search. Access via email-challenge auth with PoW,
Cedar authorization, PDF download with short-lived tokens.

This README is the constitution: its rules are mandatory. Read it, `AGENTS.md`
(skills, tool restrictions), and `document/` before any work — they
cross-reference and must be read together.

## 1. Layout

- `code/` — workspace: `back` (axum), `front` (Leptos CSR), `common` (shared),
  `proxy` (pingap).
- `configuration/` — runtime toml; `proxy/` holds pingap config.
- `data/` — agdb DB, SeekStorm index, PDF storage. Dev DB resets/reseeds at
  startup; deleting it forces fresh init.
- `log/` — backend and proxy logs.
- `document/` — docs: `workflow.md`, `handoff/`, `run.md`.
- `test/` — shared unit tests pulled in via `#[path]`.

## 2. Architecture

Layering mandatory; dependencies point strictly inward.

Backend: `interface → logic`, `interface/logic/repository → infrastructure`,
`logic → repository`.

- `main.rs` — composition root: config, logging, seed, start server.
- `interface` — axum routes: one `<verb>_<resource>` handler per route.
- `logic` — business rules, authorization, search, pagination.
- `repository` — agdb, SeekStorm, moka cache.
- `infrastructure` — config, logging, email, PDF, server bootstrap.

Frontend: `router → page → request`, `page/request → infrastructure`.

- `main.rs` — composition root: runtime-config signals, mounts router.
- `router` — URL → `page` only.
- `page` — UI + local state; calls backend only via `request`.
- `request` — all HTTP, session tokens, `{code, data, message}` unwrap.
- `infrastructure` — wasm primitives (compile-time config, runtime config fetch,
  storage, PoW).

`common` — shared data structures/methods (hash, PoW, name, tag, text, search,
time, request/response). Both depend on it; it depends on nothing internal.

Module org: never `mod.rs`; module = same-named `.rs` + folder; deepen if a dir
exceeds 16 files.

## 3. Stack

Frontend: Leptos CSR via trunk; proxy: pingap (static + reverse `/api/*`).
Backend: axum, agdb, SeekStorm, moka, cedar-policy, lettre, tokio, tracing.
Hashing: ascon; IDs/tokens: UUIDv7. Versions pinned by each crate's
`Cargo.lock`; pinned sources in `~/.cargo/registry/src/index.crates.io-*/`.

## 4. Coding standards

- **Language**: English only — code, docs, comments, UI.
- **Naming**: no abbreviations (loop vars `i`/`j`/`k` excepted). Backend:
  CRUD-only verbs (`create`/`read`/`update`/`delete`) for every resource;
  collection reads are `read` (never `list`), paginated. Node ops, not flow
  vocabulary (no `intent=authenticate`). `interface` strictest;
  `repository`/`infrastructure` keep their own precise terms.
- **Size**: file ≤ 512 lines; function ≤ 256 lines.
- **General**: concise; prefer pure functions; no hardcoding (toml); no dead
  code; zero-warning gate on every build.
- **Comments**: only for non-obvious intent/constraints/tradeoffs; restating the
  code is a defect.

## 5. Robustness & security

- Panic-free: never `unwrap`/`expect`.
- Errors propagate with `?`; convert only at layer boundaries; interface maps
  the final error to the `{code, data, message}` envelope.
- IDs/tokens: UUIDv7; hashing: ascon only.
- Authorization enforced in `logic` against Cedar policies; every request goes
  through a principal session.

## 6. Configuration

Toml under `configuration/`, never hardcoded. `server.toml` (runtime, read at
startup), `front.toml` (compile-time via `include_str!`, fail fast),
`email.toml` (allowed domains), `smtp.toml` (secrets, gitignored; committed
template `smtp.toml.example`). Backend serves `/config/read` for the frontend.

## 7. Backend rules

Every response is `{code, data, message}` (code=status, message=reason,
data=payload). Logging: `tracing` + `tracing-subscriber` to `log/`, daily
pruning.

## 8. Frontend rules

Leptos CSR. Deployment params embedded at compile time, fail
fast; other config fetched at runtime from `/config/read` (compile-time defaults
until first fetch); backend stays authoritative.

## 9. Design order

Define data structures first (request/response payloads, DB node/edge shapes,
cache layout), then the logic.

## 10. Testing

Test every function across all cases (exhaustive when cheap; else boundaries +
randomized regular cases). Unit tests in `test/unit/{common,back,front}` via
`#[path]`. Run `cargo test` in each crate; keep `cargo clippy` (zero warnings)
and `cargo fmt` clean. Clippy runs plain (no `--all-targets`) so tests are
exempt.

## 11. Building & running

Full-stack restart: `document/run.md`. Backend alone:
`cargo run --bin nail_back` (from `code/back`); seed samples with
`-- seed-samples [count]`. Frontend: `trunk build` (from `code/front`), served
by the proxy.

## 12. Dependencies

Add one by one with `cargo add`, alphabetical, latest non-conflicting; commit
`Cargo.lock`. For any crate question: read the pinned source, then confirm with
a probe test — source + probe evidence, never a guess. No implementation until
both are recorded and the user adopts the plan (`document/workflow.md` §7).

## 13. Documentation

`workflow.md` = execution loop; `handoff/` = progress (per-task files +
`readme.md` index); `run.md` = build/run/health-check.

## 14. Search scope

Search is precise, never the whole tree: most of the tree is generated or
runtime data and would pollute results. Always set the search root; never
search the repo root or a whole crate.

Relevant roots (search one of these, not above them):

- `code/{back,front,common}/src/` — Rust source.
- `test/`, `document/`, `configuration/` — those layers only.
- `code/{back,front,common}/Cargo.toml` — dependency declarations.

Never include (large or unrelated; pollutes results):

- `target/` — root and every crate (combined ~14G).
- `code/front/dist/` — frontend build output.
- `code/proxy/pingap-linux-gnu-x86-full` — downloaded binary.
- `data/`, `log/`, `.git/`, `Cargo.lock`.

For a repo-root `rg`, exclude all of the above:
`rg -g '!target' -g '!dist' -g '!data' -g '!log' -g '!*.lock'
-g '!pingap-linux-gnu-x86-full'`. A `src/` root needs no exclusions.