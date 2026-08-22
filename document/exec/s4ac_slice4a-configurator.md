# Exec s4ac slice4a-configurator

## Requirement — R
Collapse Configurator wrapper without behavior change. Acceptance: no configurator indirection in AppState hot path; all call sites use state.config.*; tests pass; fmt/clippy clean.

## Scope
In: infrastructure/state.rs, infrastructure/config.rs/server.rs (if needed), all logic/*, interface/*, infrastructure/server.rs, tests/context.rs
Out: target/, dist/, data/, log/, Cargo.lock

## Design decisions
- AppState holds `pub config: Arc<AppConfig>` instead of `configurator: Configurator`.
- Keep `Configurator` struct with Deref to AppConfig + deprecated forwarders for compat, or type alias; update internal call sites to `state.config.xxx` / `state.config.server.xxx`.
- `challenge::create_challenge` currently takes &Configurator; change to take pow_iterations or &AppConfig; keep deprecated wrapper overload via Deref.
- Minimal change: AppState.config, Configurator Deref impl.

## Slice breakdown
1. Slice 4a: Replace AppState field, add Deref, update all call sites, keep deprecated wrappers.

Files: code/server/src/infrastructure/state.rs, code/server/src/infrastructure/server.rs, code/server/src/logic/*, code/server/src/interface/*, code/server/src/tests/context.rs

Red: before change tests pass; after incomplete rename tests fail to compile.
Green: all call sites use config, tests pass.
Exit test: cargo test -j1 -p server -p common ; cargo fmt --check ; cargo clippy -p server -p common -- -D warnings

## Open unknowns
- None.

## Verification plan
- Probe grep configurator count before/after.
- cargo test -j1 -p server -p common
- cargo clippy + fmt

## Risks
- Missed call site -> compile fail (caught).
- Behavior change if field semantics differ -> none, pure getters.

## Constraints
- No Cargo.lock edit, no unwrap, one commit, clean tree, reuse baseline e432555.

## Questions
- None.
