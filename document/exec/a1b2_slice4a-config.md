# Exec slice4a-config
1. Requirement: as above
2. Scope: infrastructure/state.rs, logic/*, interface/*
3. Design: delete Configurator, Deref impls
4. Slices: single slice
5. Open: none
6. Verification: cargo clippy, cargo test -p server
7. Risks: none
8. Constraints: net -20 lines
9. Questions: none
