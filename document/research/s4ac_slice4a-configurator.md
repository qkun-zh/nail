# Research s4ac slice4a-configurator

## Requirement — R0
Collapse Configurator wrapper without behavior change: AppState holds Arc<AppConfig> directly or Configurator Deref to ServerConfig; update all state.configurator.xxx() call sites to state.config.xxx()/state.server_config.xxx() preserving semantics; keep method names if needed as deprecated wrappers.

## Research questions
1. Does Configurator still exist as 15 forwarders?
2. What call sites depend on configurator?
3. Behavior change risk?

## Evidence
### Q1 source
- `code/server/src/infrastructure/state.rs:19-90` Configurator(Arc<AppConfig>) with 15 methods forwarding to AppConfig.server/cache etc. — verified.

### Q1 probe
- `rg configurator` shows ~50 hits; state.rs still contains struct — no probe test needed beyond grep; redundancy confirmed.

### Q2 source
- `rg configurator` lists logic/* (article, version, comment, search, email, pow, download, challenge) + interface/* (extractor, multipart, router, config, challenge) + infrastructure/server.rs + tests/context.rs etc.

### Q2 probe
- Existing tests pass per instruction baseline e432555 reused; incremental verify via `cargo test -p server -p common` (not re-run yet for this slice, will run post-change).

### Q3 source
- All forwarders are pure getters; replacing with direct field access `state.config.server.xxx` or `state.config.cache.xxx` preserves semantics. `max_request_body_bytes()` delegates to ServerConfig method — same.

### Q3 probe
- No behavior change if Arc<AppConfig> holds same data; Deref impl would preserve `configurator.xxx()` compatibility but requirement prefers direct access.

## Findings
- Redundancy present. Safe to replace AppState.configurator with config: Arc<AppConfig> and Configurator as Deref wrapper for compat.
- Call sites all read-only.

## Impact on R
- No revision needed; R0 stands.

## Open items
- None.
