# Project Setup & Tooling

> Unit `01-project-setup` · kind: project-setup

## Summary

Defines the build/tooling contracts of the reconstructed `nail` application: the three Cargo manifests (`code/back/Cargo.toml`, `code/common/Cargo.toml`, `code/front/Cargo.toml`) that declare the crate identities, editions, and the complete dependency inventory with exact versions and feature sets, plus the repository hygiene files (`.gitignore`, `LICENSE`). This unit produces a skeleton that compiles on the first `cargo build` and whose dependency graph exactly matches the original.

## Context & goal

Context: the original `nail` project is a three-crate Rust codebase — a `nail_back` server binary (axum 0.8, embedding the agdb graph database, the SeekStorm search engine, and the moka cache), a `nail_front` Leptos CSR (client-side rendered) wasm binary, and a shared `common` library crate that both depend on. The three `Cargo.toml` files carry the entire third-party surface of the application: every `use` in the 120 source files of unit `02-code` resolves to a dependency declared here.

Goal (user-facing): a developer can clone the rebuilt repo and, from the three manifests alone, run `cargo build` and `cargo test` on each crate and get the exact original compile surface — same crates, same versions, same features, same feature-gated test tooling — with nothing heavy or secret leaking into the wrong build.

Dependencies: none (unit `01` is the foundation of the reconstruction tree; no other unit is required to produce these manifests). Units that depend on it: `02-code` — every source file compiles inside the crates this unit defines; downstream units inherit the dependency inventory from here.

## User stories

- As a developer, I can run `cargo build` in `code/back` with no extra flags, so that the production server binary compiles without pulling the heavy end-to-end dependency tree.
- As a developer, I can run `cargo test` in `code/back`, so that unit tests compile and run while the end-to-end-only dependencies (chromiumoxide, reqwest, futures) are absent from the build.
- As a developer, I can run `cargo test --features end_to_end` in `code/back`, so that the full end-to-end suite (real TCP/HTTP, in-process SMTP sink, browser scenario) compiles and runs.
- As a developer, I can build `code/common`, so that the shared data structures, ascon-family hashing, and PoW utilities compile as a library with no internal dependencies.
- As a developer, I can build `code/front` for the wasm target, so that the Leptos CSR bundle compiles with the exact `web-sys` feature surface.
- As a developer of `back` and `front`, I can depend on `common` by path (`../common`), so that shared types are defined once and reused by both binaries.
- As a developer, I can rely on all three crates using edition 2024, so that modern Rust syntax and std behavior apply uniformly.
- As the rebuild maintainer, I can verify each declared dependency (name, version, feature list) against the original manifests, so that the rebuild behaves identically (agdb 0.13.2, seekstorm 3.3.5 with default-features off, axum 0.8, leptos 0.8.19, uuid 1.23.4, ...).
- As the rebuild maintainer, I can verify the dependency inventory is complete and minimal, so that nothing the `02-code` sources use is missing and nothing unused is present.
- As a git user of the repo, I can rely on `.gitignore`, so that build artifacts (`target/`, `dist/`), runtime data (`data/`), logs (`log/`, `*.log`), and secrets (`conf/back/smtp.toml`, `conf/imap.toml`, `.env`) are never committed.
- As a downstream user, I can rely on the MIT license (© 2026 qkun-zh), so that I know my rights to copy, modify, and distribute the software.
- As a developer, I can run the end-to-end suite only when I explicitly opt in, so that default builds and tests stay fast and avoid compiling chromiumoxide's heavy tree.

## Functional requirements

Tagged `[confirmed]` (read directly in the manifests / reference repo files), `[inferred]` (derived from patterns, no false certainty), `[gap]` (needs a human decision).

### Package identity

1. [confirmed] `code/back/Cargo.toml` `[package]` declares `name = "nail_back"`, `version = "0.1.0"`, `edition = "2024"`.
2. [confirmed] `code/common/Cargo.toml` `[package]` declares `name = "common"`, `version = "0.1.0"`, `edition = "2024"`.
3. [confirmed] `code/front/Cargo.toml` `[package]` declares `name = "nail_front"`, `version = "0.1.0"`, `edition = "2024"`.

### `common` crate (shared library)

4. [confirmed] `common` depends on nothing internal (no path dependencies) and exactly: `anyhow = "1.0.103"`, `ascon-xof128 = "0.2.1"`, `hex = "0.4.3"`, `pso-vdf = "0.2.2"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `tracing = "0.1"`, `uuid = { version = "1.23.4", features = ["serde", "v4", "v7"] }`.
5. [confirmed] `common` has no `[features]` and no `[dev-dependencies]` sections; its `uuid` enables exactly the features `serde`, `v4`, `v7`.

### `nail_back` crate (server binary)

6. [confirmed] `nail_back` declares `common = { path = "../common" }` as its only internal dependency.
7. [confirmed] `nail_back` regular dependencies are exactly (20 crates, excluding `common` and the 3 optional ones): `toml = "0.8"`; `serde = { version = "1", features = ["derive"] }`; `uuid = { version = "1.23.4", features = ["serde", "v4", "v7"] }`; `tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time", "signal"] }`; `agdb = "0.13.2"`; `seekstorm = { version = "3.3.5", default-features = false }`; `cedar-policy = "4.12"`; `serde_json = "1"`; `axum = { version = "0.8", features = ["multipart"] }`; `anyhow = "1.0.103"`; `hex = "0.4.3"`; `lettre = "0.11.22"`; `email_address = "0.2.9"`; `semver = "1"`; `moka = { version = "0.12.15", features = ["sync"] }`; `rand = "0.9"`; `tracing = "0.1"`; `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`; `chrono = "0.4"`; `tokio-util = { version = "0.7", features = ["io"] }`.
8. [confirmed] `agdb` is pinned at `0.13.2` (fact graph data: nodes, edges, transactions) and `seekstorm` at `3.3.5` with `default-features = false` (search inverted index; derived data; the default pdfium/zh features are excluded).
9. [confirmed] `axum` is pinned at `0.8` with exactly the `multipart` feature (multipart uploads).
10. [confirmed] `moka` is `0.12.15` with the `sync` feature (cache).
11. [confirmed] `cedar-policy` is `4.12` (authorization policy engine; the Cedar schema is unit `02-code`).
12. [confirmed] `lettre` is `0.11.22` and `email_address` is `0.2.9` (email sending and address validation).
13. [confirmed] `tokio` 1.x enables exactly `rt-multi-thread`, `macros`, `sync`, `time`, `signal`; `tokio-util` 0.7 enables `io`.
14. [confirmed] `tracing` 0.1 and `tracing-subscriber` 0.3 with the `env-filter` feature (logging; runtime logs go to the `log/` directory).
15. [confirmed] Access logging, rate limiting, and request-body size limits are NOT part of the manifest: `tower_governor` and the `tower-http` trace feature are absent because those duties moved to the pingap reverse proxy (source comment, back Cargo.toml).
16. [confirmed] `end_to_end` is a declared Cargo feature, default off, equal to `["dep:reqwest", "dep:chromiumoxide", "dep:futures"]`.
17. [confirmed] `reqwest` is an optional dependency: version `0.12`, `default-features = false`, features `json`, `multipart`, `rustls-tls`, `stream`.
18. [confirmed] `chromiumoxide` is an optional dependency, version `0.9.1`; `futures` is an optional dependency, version `0.3`.
19. [confirmed] The three end-to-end dependencies are optional NORMAL dependencies (not dev-dependencies), gated by the `end_to_end` feature — the source comment explains Cargo does not allow optional dev-dependencies, so a plain `cargo build`/`cargo test` does not compile chromiumoxide or reqwest's heavy trees.
20. [confirmed] `nail_back` dev-dependencies are exactly: `tower = { version = "0.5", features = ["util"] }`, `tower-http = { version = "0.6", features = ["fs"] }`, `tokio = { version = "1", features = ["io-util", "net", "macros"] }` (used by end-to-end tests: `tower::ServiceExt::oneshot` against real axum routes, static-file serving of `code/front/dist` via tower-http fs, in-process fake SMTP server via `TcpListener`).

### `nail_front` crate (CSR wasm binary)

21. [confirmed] `nail_front` declares `common = { path = "../common" }` as its only internal dependency.
22. [confirmed] `nail_front` regular dependencies are exactly (16 crates, excluding `common`): `anyhow = "1.0.103"`, `toml = "0.8"`, `serde = { version = "1", features = ["derive"] }`, `console_error_panic_hook = "0.1.7"`, `gloo-net = "0.7.0"`, `gloo-storage = "0.4.0"`, `gloo-timers = "0.3.0"`, `hex = "0.4.3"`, `leptos = { version = "0.8.19", features = ["csr"] }`, `leptos_router = "0.8.13"`, `serde_json = "1"`, `js-sys = "0.3"`, `wasm-bindgen = "0.2"`, `wasm-bindgen-futures = "0.4"`, `web-sys = { version = "0.3", features = [19 web-sys features, see req 25] }`, `uuid = { version = "1.23.4", features = ["serde", "v4", "v7", "js"] }`.
23. [confirmed] `leptos` is `0.8.19` with exactly the `csr` feature (client-side rendering).
24. [confirmed] `leptos_router` is `0.8.13` (must stay on the 0.8 line matching leptos).
25. [confirmed] `web-sys` 0.3 enables exactly these 19 features: `Blob`, `File`, `FileList`, `Url`, `HtmlInputElement`, `HtmlAnchorElement`, `FormData`, `AbortController`, `AbortSignal`, `Worker`, `WorkerOptions`, `WorkerType`, `MessageEvent`, `HtmlScriptElement`, `HtmlCollection`, `Element`, `Node`, `Window`, `Document`.
26. [confirmed] `uuid` in the front crate enables `serde`, `v4`, `v7`, and additionally `js` (wasm-compatible UUID).
27. [confirmed] `nail_front` has no `[features]` and no `[dev-dependencies]` sections.

### Repository hygiene

28. [confirmed] `.gitignore` (reference repo root) ignores exactly: `target/`, `dist/` (build artifacts); `data/` (runtime graph DB + uploaded PDFs); `log/` (runtime logs); `conf/back/smtp.toml`, `conf/imap.toml`, `.env` (secrets); `*.log`; `code/proxy/pingap-linux-gnu-x86-full` (downloaded tooling); `.vscode/`, `.idea/`, `.DS_Store`, `*.rs.bk`, `__pycache__/` (editor/misc).
29. [confirmed] `LICENSE` (reference repo root) is the MIT License, copyright line `Copyright (c) 2026 qkun-zh`.
30. [inferred] The three crates are standalone manifests (no `[workspace]` table anywhere in the reference): `back` and `front` reach `common` purely through `path = "../common"`.
31. [inferred] With the `end_to_end` feature off, `cargo tree -e normal` for `nail_back` must not contain `chromiumoxide`, `reqwest`, or `futures`.
32. [inferred] Building `nail_front` requires the `wasm32-unknown-unknown` target to be installed; the built CSR bundle is what the end-to-end browser scenario serves from `code/front/dist` (referenced by the tower-http fs dev-dependency comment).
33. [gap] The inventory description for this unit mentions build, lint, env, and CI, but the reference contains no lint configuration (rustfmt/clippy defaults apply) and no CI pipeline files — CI is not part of the source and must be designed, not copied.

## Interfaces & data

This unit exposes no runtime interfaces: no HTTP routes, no RPC operations, no jobs, no external-service calls, no entities, and no enums. Its entire contract surface is the **build interface** — the crate graph and the dependency inventory — which unit `02-code` consumes by compiling inside these crates. No interface rows are proposed for `architecture/INTERFACES.md` and no entity rows for `architecture/DATA-MODEL.md`.

Build contract (crate graph):

| Crate (manifest path) | Kind | Depends on | Feature flags | Role |
| --- | --- | --- | --- | --- |
| `code/common/Cargo.toml` — `common` | library | std + 8 crates, no internal deps | none | shared data structures, ascon-family hashing, PoW (pso-vdf), request/response types, time/name/search/text utils |
| `code/back/Cargo.toml` — `nail_back` | binary (server) | `common` (path `../common`) + 20 regular crates + 3 e2e-gated optional crates | `end_to_end` (default off) | axum API server; agdb graph DB; SeekStorm search; moka cache; Cedar authorization; lettre email; tracing logging |
| `code/front/Cargo.toml` — `nail_front` | binary (Leptos CSR wasm) | `common` (path `../common`) + 16 crates | none | client-side rendered UI; gloo-net HTTP; gloo-storage/gloo-timers; web-sys/wasm-bindgen bindings; browser PoW |

Write contract: none — this unit writes no data, no entities, and no files at runtime. The only writes are build artifacts (`target/`, `dist/`), which `.gitignore` excludes. Env vars: none declared in the manifests (`inventory.envVars` is empty); configuration handling is a runtime concern of `02-code`.

## Acceptance criteria

Given/When/Then scenarios; at least one per functional requirement, including failure paths.

### Package identity

- Given a fresh checkout of the rebuilt repo, When I run `cargo metadata --format-version 1 --no-deps` in each crate, Then the reported name/version/edition are `nail_back`/`0.1.0`/`2024`, `common`/`0.1.0`/`2024`, and `nail_front`/`0.1.0`/`2024` respectively (reqs 1–3).
- Given a manifest with a wrong edition or name, When the crate is built, Then the build fails and the manifest must be corrected before any source lands (reqs 1–3, failure path).

### `common` crate

- Given the rebuilt `code/common/Cargo.toml`, When its `[dependencies]` are diffed against the ground truth, Then every dependency name, version requirement, and feature list matches and no extra or missing entry exists (req 4).
- Given `common` built, When I inspect `cargo metadata`, Then `common` declares no path or workspace dependency (reqs 4, 30).
- Given the manifest, When I check the `uuid` declaration, Then its features are exactly `serde`, `v4`, `v7` and no `[features]`/`[dev-dependencies]` sections exist (req 5).

### `nail_back` crate

- Given the rebuilt `code/back/Cargo.toml`, When its `[dependencies]`, `[features]`, and `[dev-dependencies]` are diffed against the ground truth, Then each entry matches in name, version requirement, and features, with no additions or removals (reqs 6–20).
- Given a clean default build of `nail_back`, When I run `cargo tree -e normal`, Then the graph contains `common` (path) and the 20 regular crates and does NOT contain `chromiumoxide`, `reqwest`, or `futures` (reqs 17–19, 31).
- Given `seekstorm` declared with `default-features = false`, When I run `cargo tree -e features`, Then no `pdfium` feature is enabled (req 8, failure prevention).
- Given `agdb` declared as `"0.13.2"`, When Cargo resolves it, Then the resolved version satisfies `>=0.13.2, <0.14.0` (req 8).
- Given the manifest, When I check the `end_to_end` feature, Then it expands to exactly `["dep:reqwest", "dep:chromiumoxide", "dep:futures"]` and is not in the default set (reqs 16–19).
- Given `end_to_end` NOT passed, When I run `cargo test` in `code/back`, Then unit tests compile and run and the end-to-end test modules are not compiled (reqs 16, 19).
- Given `cargo test --features end_to_end` in `code/back`, Then the end-to-end suite compiles and runs — real HTTP through `tower::ServiceExt::oneshot`, SMTP sink over `TcpListener`, static files via tower-http fs (reqs 19–20).
- Given the `end_to_end` feature enabled but the SMTP-sink port already in use, When the sink test binds, Then it fails with a bind error rather than hanging (req 20, failure path).
- Given the manifest, When I check the `axum` declaration, Then version is `0.8` and features are exactly `["multipart"]` (req 9).
- Given the manifest, When I check `tracing-subscriber`, Then version is `0.3` and features are exactly `["env-filter"]` (req 14).
- Given the manifest, When I search for `tower_governor` and the `tower-http` trace feature, Then they are absent (those duties moved to the pingap reverse proxy) (req 15).

### `nail_front` crate

- Given the rebuilt `code/front/Cargo.toml`, When its `[dependencies]` are diffed against the ground truth, Then every entry matches in name, version, and features (reqs 21–27).
- Given the manifest, When I check `leptos`, Then version is `0.8.19` and features are exactly `["csr"]` (req 23).
- Given the manifest, When I check `web-sys`, Then its feature list is exactly the 19 features of req 25, no more and no less (req 25).
- Given the manifest, When I check `uuid`, Then features are exactly `serde`, `v4`, `v7`, `js` (req 26).
- Given the `wasm32-unknown-unknown` target installed, When I run `cargo build --target wasm32-unknown-unknown` in `code/front`, Then it compiles and emits the CSR bundle (reqs 22, 32).
- Given the wasm target NOT installed, When the front is built, Then the build fails with a clear target-not-installed error and `rustup target add wasm32-unknown-unknown` is the documented fix (req 32, failure path).

### Repository hygiene

- Given the rebuilt `.gitignore`, When I run `git check-ignore -v` on `target/x`, `dist/x`, `data/x`, `log/x`, `conf/back/smtp.toml`, `conf/imap.toml`, `.env`, `x.log`, `code/proxy/pingap-linux-gnu-x86-full`, `.vscode/x`, `.idea/x`, `.DS_Store`, `x.rs.bk`, and `__pycache__/x`, Then each is reported as ignored by the corresponding pattern (req 28).
- Given a file not in the ignore list (e.g., `README.md`, a source file), When I run `git check-ignore`, Then it is NOT ignored (req 28, negative case).
- Given a secret file in the working tree, When `git status` is inspected, Then it never appears as a candidate because `.gitignore` matched it (req 28).
- Given the rebuilt repo root, When I inspect it, Then `LICENSE` exists and its text is the MIT License with the copyright line `Copyright (c) 2026 qkun-zh` (req 29).

## Edge cases & failure modes

| # | Situation | Expected behavior | Maps to |
| --- | --- | --- | --- |
| E1 | Cargo resolves a caret requirement (`axum 0.8`, `agdb 0.13.2`) to a future version with changed API | Resolution stays within the declared semver range; for `0.x` deps it must not jump minor lines; the ground-truth pins are the verified floor | reqs 7–14 |
| E2 | `seekstorm` default features accidentally re-enabled | The `pdfium`/`zh` feature trees compile, bloating and slowing the build; keep `default-features = false` exactly | req 8 |
| E3 | An e2e-only dep (`reqwest`, `chromiumoxide`, `futures`) moved from optional to plain | Every `cargo build`/`cargo test` compiles the heavy tree; keep them optional and feature-gated | reqs 16–19 |
| E4 | `cargo add` (target rule) floats versions past the verified pins | Behavior changes silently; keep the ground-truth pins as the floor and re-verify by building `02-code` | redesign §9 |
| E5 | Missing `wasm32-unknown-unknown` target | Front build fails with a rustup target error; documented, non-code failure | req 32 |
| E6 | Path dependency `../common` resolved from the wrong directory | Back/front builds fail or link a stale crate; the skeleton is fixed, so the relative path is stable | reqs 6, 21 |
| E7 | End-to-end SMTP sink port already in use | The e2e test fails with a bind error; no retry or hang | req 20 |
| E8 | A secret file committed before `.gitignore` exists | Secrets end up in history; `.gitignore` must be created in this unit, before any commit | req 28 |
| E9 | A dependency used by `02-code` sources is missing from the manifests | Compile error in the consuming crate; the inventory must be complete before source lands | reqs 4, 7, 22 |
| E10 | An unused dependency left in a manifest | Build still succeeds but violates the no-dead-code rule; prune with an unused-dependency check | redesign notes |
| E11 | Chinese comments retained in the manifests | Violates the target English-only rule; translate to English | redesign notes |
| E12 | Non-alphabetical dependency order in the manifests | Violates the target cargo-add-alphabetical rule; sort the sections | redesign notes |
| E13 | A `[workspace]` table added at the root | Changes resolution semantics (shared lockfile, feature unification) vs the original standalone layout; do not add | req 30 |

## Test plan (write these first)

Write these as red tests/commands before implementing the manifests; the manifests are the implementation.

- [ ] Manifest contract test: for each of the 3 manifests, assert package name/version/edition and the exact dependency map (name, version requirement, feature list, optional flag) against the ground truth (scripted diff or assertions over `cargo metadata` output).
- [ ] `cargo build` in `code/common` (default) — green.
- [ ] `cargo build` in `code/back` (default) — green; `cargo tree -e normal` excludes `chromiumoxide`/`reqwest`/`futures`.
- [ ] `cargo test` in `code/back` (default) — green; end-to-end modules not compiled.
- [ ] `cargo test --features end_to_end` in `code/back` — compiles and runs (HTTP oneshot, SMTP sink, fs hosting).
- [ ] `cargo build --target wasm32-unknown-unknown` in `code/front` — green (document the target requirement).
- [ ] `cargo metadata` assertions: `nail_back` depends on `common` (path); `nail_front` depends on `common` (path); no `[workspace]` in the tree.
- [ ] Feature-flag assertions: `end_to_end` expands to exactly `["dep:reqwest","dep:chromiumoxide","dep:futures"]` and is not default; `seekstorm` has `default-features = false`; `leptos` features are `["csr"]`; `web-sys` feature list equals the 19 members; `uuid` features differ per crate (`serde,v4,v7` vs `serde,v4,v7,js`).
- [ ] `.gitignore` test: `git check-ignore` returns the matching pattern for each of the 14 entries and nothing for a control file.
- [ ] `LICENSE` test: file exists and contains `MIT License` and `Copyright (c) 2026 qkun-zh`.
- [ ] Negative test: temporarily introduce a manifest typo (missing feature, wrong version) and assert the contract test fails (proves the test catches regressions).

## Source material

Files that implement this unit (rewrite them from the requirements above):

- `code/back/Cargo.toml`
- `code/common/Cargo.toml`
- `code/front/Cargo.toml`

Ground truth: copies under `/home/qkun/nail_new/document/reconstruction/data/config/code/{back,common,front}/Cargo.toml`, byte-identical to `/home/qkun/nail_new/document/reference/code/...` (including the Chinese comments). In-scope companion files from the reference repo root: `.gitignore`, `LICENSE`.

## Improvements & refactors

Each item tagged so the rebuild stays safe by default:

- [ ] [keep-behavior] Translate the Chinese comments in `code/back/Cargo.toml` (the end-to-end gating rationale, the agdb/seekstorm storage-stack rationale, the tower-http fs rationale) into English — mandated by the target English-only rule; the comments carry the why of the feature gate.
- [ ] [keep-behavior] Sort `[dependencies]`/`[dev-dependencies]` alphabetically in all three manifests, matching the target rule that dependencies are added one by one with `cargo add` in alphabetical order.
- [ ] [keep-behavior] Rename the `common` crate package to `nail_common` for naming consistency with `nail_back`/`nail_front` (mandated by the target skeleton rule; the folder stays `code/common`, the path dependency becomes `nail_common = { path = "../common" }`, and `use common::...` in `02-code` sources becomes `use nail_common::...`).
- [ ] [keep-behavior] Document the exact bootstrap sequence (skeleton first, then `cargo add` per dependency, alphabetical, with the ground-truth pins as the minimum version) in the rebuild instructions.
- [ ] [keep-behavior] Add a manifest lint step (unused-dependency check or a pinned-version compare script) so reqs 4/7/22 cannot silently drift.
- [ ] [keep-behavior] Keep `end_to_end` opt-in exactly as in the source; keep an English comment explaining why the e2e deps are optional normal deps rather than dev-deps (Cargo does not support optional dev-dependencies).
- [ ] [behavior-change] (opt-in) Add a CI pipeline (e.g., `cargo check` per crate, `cargo test --features end_to_end`, wasm build) — nothing exists in the source; design it, do not copy it.
- [ ] [behavior-change] (opt-in) Add supply-chain checks (`cargo audit` / `cargo deny`) on a schedule — new tooling; may block builds on advisories.
- [ ] [behavior-change] (opt-in) Enforce `cargo fmt --check` and clippy lints in the default test path — no lint config exists in the source; the reference relies on tool defaults.

## Redesign notes

Mapping onto the target project rules in `/home/qkun/nail_new/README.md`:

- **§3 fixed skeleton** — the three crate folders `code/back`, `code/common`, `code/front` each keep a root `Cargo.toml` plus `src/`. The skeleton is fixed; only the `common` crate's `src/` submodule placeholders (`zzz`/`yyy`/`xxx`) are free to rename. `code/common` remains the folder name even though the crate package is renamed to `nail_common`.
- **§9 build & dependencies** — scaffold the top-level structure first; then add every dependency one by one with `cargo add`, in alphabetical order, always the latest non-conflicting versions. The ground-truth pins are the verified floor: `agdb 0.13.2`, `axum 0.8`, `seekstorm 3.3.5 (default-features=false)`, `leptos 0.8.19 (csr)`, `leptos_router 0.8.13`, `uuid 1.23.4`, `moka 0.12.15 (sync)`, `cedar-policy 4.12`, `lettre 0.11.22`, `anyhow 1.0.103`, `gloo-net 0.7.0`, `gloo-storage 0.4.0`, `gloo-timers 0.3.0`, `console_error_panic_hook 0.1.7`, `ascon-xof128 0.2.1`, `pso-vdf 0.2.2`. `cargo add` may float to newer compatible versions; keep `0.x` dependencies within the same minor line and re-verify by building unit `02-code`.
- **Edition 2024** — all three crates, matching the source and the target rule.
- **Three independent crates** — standalone manifests, no `[workspace]`; `nail_back` and `nail_front` depend on `nail_common` by path `../common`; `nail_common` depends on nothing internal (§4.3).
- **§5.1 English-only** — the source manifests carry Chinese comments; they are translated to English or removed.
- **§5.4 no dead code / §9 minimal inventory** — every declared dependency must be used by `02-code` source; nothing extra is added.
- **§2 stack alignment** — back: `axum` + `agdb` + `seekstorm` + `moka` (present in the manifest); front: `leptos` CSR (feature `csr`, present); proxy `pingap` is external — its duties (access log, rate limit, body-size cap) are intentionally absent from the manifest per the source comment; do not re-add `tower_governor` or the `tower-http` trace feature.
- **§6 robustness/security** — `uuid` `v7` (search IDs and tokens), `ascon-xof128` (all hashing), `pso-vdf` (PoW), `anyhow` (error propagation) are all present in the inventory.
- **§8 logging** — `tracing` + `tracing-subscriber` (env-filter) present; runtime logs go to the target `log/` directory.
- **§10 frontend rules** — `toml` is present in `nail_front` for compile-time embedded configuration (deployment parameters), matching the embed-at-compile-time-from-toml rule; runtime config fetching belongs to `02-code`.
- **§11 backend rules** — `toml` is present in `nail_back` for startup configuration; the `{code, data, message}` envelope is a runtime concern of `02-code`.
- **`.gitignore` remap** — the source patterns reference the original layout (`conf/back/smtp.toml`, `conf/imap.toml`, `code/proxy/pingap-linux-gnu-x86-full`). The target skeleton replaces `conf/` with `configuration/` and pingap becomes an external tool; the secret-file patterns must be remapped to the target config layout in the configuration unit — flagged as a gap (see notes).
- **LICENSE** — keep the MIT text and the original copyright line `Copyright (c) 2026 qkun-zh` unless the owner directs otherwise.

## Definition of done

- [ ] Every functional requirement (1–33) is satisfied by the three manifests and the hygiene files; each `[confirmed]` matches the ground truth, each `[inferred]` holds under the build, and each `[gap]` is resolved in the notes or handed to a human.
- [ ] Every acceptance-criteria scenario passes, including the failure paths.
- [ ] No runtime interfaces or entities are introduced (this unit exposes none).
- [ ] `cargo build`/`cargo test` succeed in `code/common` and `code/back`; `cargo test --features end_to_end` succeeds in `code/back`; the front compiles for `wasm32-unknown-unknown`.
- [ ] `cargo tree -e normal` for `nail_back` excludes `chromiumoxide`/`reqwest`/`futures` under default features.
- [ ] `.gitignore` covers all 14 original patterns; `LICENSE` carries the MIT text and the copyright line.
- [ ] Redesign notes applied: alphabetical dependency order, English comments, crate renamed to `nail_common` with path dependencies updated.
- [ ] `node scripts/analyze.mjs --check --out <out>` passes — no unresolved agent callouts or placeholders, and every reference resolves.
