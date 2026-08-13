# reference — reconstruction summary

> Generated with `reconstruct@2.17.0` · mode `redesign` · level `complex` · fidelity `describe`

## Project
- **Stack:** Rust
- **Notable libraries:** —
- **Size:** 205 files · 31830 lines
- **Routes:** 0 · **Features:** 3

## Features (build order)
1. **Project Setup & Tooling** — 3 configuration/tooling file(s): build, lint, env, CI. → `features/01-project-setup/PRD.md` (3 file(s))
2. **Code** — Groups 120 file(s). → `features/02-code/PRD.md` (120 file(s))
3. **Test** — Groups 70 file(s). → `features/03-test/PRD.md` (70 file(s))

## Interface & data surface
- Routes resolved: 0
- Route candidates to verify: 37
- API candidates (RPC / GraphQL / gRPC / OpenAPI): 0
- Schema / data-model candidates: 3

## Unknowns to resolve
- No web framework was detected from manifests — identify the stack from `stack.languages` + `dependencies`, find the entry points (`hints.entryPoints`, else the file tree), then map the interface surface manually. If there is no web framework because this is a library / CLI / SDK / engine, that is a first-class case: the interface surface is the exported public API plus the CLI commands, not routes — see `references/stack-guides/library-cli-sdk.md`.
- Routes were not resolved deterministically (a framework without a dedicated route adapter, or an RPC/GraphQL surface) — derive the real interface surface from `hints.routeCandidates` / `hints.apiCandidates` into `architecture/INTERFACES.md`.
- The data model is not structured by the engine — extract entities, fields, types, and relations from `hints.schemaCandidates` into `architecture/DATA-MODEL.md`.

## Next steps
Open `REBUILD.md` for the dependency-ordered build order and validation checklist, then feed each `features/<slug>/PRD.md` to an agent, using `data/` and `source/` as ground truth.
