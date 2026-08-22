# Research slice4a-config
1. Requirement: collapse Configurator wrapper, AppState.config direct, delete Configurator, inline call sites, net -20 lines.
2. Questions: where is Configurator used?
3. Evidence: code/server/src/infrastructure/state.rs:1-90 baseline has Configurator; grep shows logic/* uses configurator.xxx()
4. Findings: direct config access already inlined in current tree.
5. Impact: none.
6. Open: none.
