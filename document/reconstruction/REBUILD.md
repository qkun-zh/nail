# REBUILD — reference

| Setting | Value |
| --- | --- |
| Mode | `redesign` |
| Level | `complex` |
| Fidelity | `describe` |
| TDD | `on` (build test-first) |
| Generated with | `reconstruct@2.17.0` |

This folder is a complete plan to rebuild the project from scratch.

## Mode & level

- **redesign**: design a new architecture for the same features.
- **complex**: PRDs that also suggest improvements to fold in.
- **describe** fidelity: descriptive PRDs only — build from requirements.
- **TDD**: each unit is built test-first (red → green → refactor).

## Build order

Ordered by dependency tier — foundations (types, data, shared UI, i18n, cross-cutting) first, feature pages next, tests & docs last.

1. [ ] **Project Setup & Tooling** → `features/01-project-setup/PRD.md`
2. [ ] **Code** → `features/02-code/PRD.md`

## Procedure

1. Start with `00-overview/PRD.md`, `architecture/ARCHITECTURE.md`, `architecture/INTERFACES.md`, and `architecture/DATA-MODEL.md`.
2. For each unit in order: write its failing acceptance tests first (red), implement until they pass (green), then refactor.
3. Wire shared data from `data/` (translations, schema, config).
4. Validate behavior against the requirements in each PRD.
5. Run the project's own scripts to verify: _no scripts detected_.

## Validation checklist

- [ ] Every interface in `architecture/INTERFACES.md` is implemented (routes, endpoints, RPC/GraphQL, jobs).
- [ ] Data model matches `architecture/DATA-MODEL.md` and `data/schema/`.
- [ ] All routes respond as before.
- [ ] Tests were written before implementation for each unit (red → green → refactor).
- [ ] Required env vars configured: _none_.
