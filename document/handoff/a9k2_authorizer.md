## Task I: authorizer standalone crate

**Owner**: x7K2pQ
**Exec doc**: `document/exec/a9k2_authorizer.md`
**Status**: Slice 1 done, Slice 2 in progress

### Stage A: Crate extraction (searcher parity)

#### Slice 1: crate skeleton
- **Goal**: standalone `authorizer` crate validates policy/schema and authorizes via snapshot.
- **Files**: `code/authorizer/*`, `code/Cargo.toml`, `test/unit/authorizer/*`
- **Status**: done — `cargo test -p authorizer` 5 passed, `clippy` clean, `fmt` clean
- **Decisions**: API `Principal/Resource` snapshot-based; build.rs codegen permissions/entities; Cedars hidden

#### Slice 2: server wiring
- **Goal**: server uses new crate via DB→snapshot adapter, behavior unchanged.
- **Files**: `code/server/src/infrastructure/state.rs`, `code/server/src/repository/authorization.rs`, `code/server/src/logic/authorize.rs`, `code/server/src/infrastructure/authorizer.rs`, `code/server/build.rs`, `code/server/Cargo.toml`
- **Status**: pending — awaiting implementation
- **Confirmations**: snapshot API adopted (user confirmed)

#### Slice 3: cleanup
- **Goal**: remove legacy baggage, promote probes, zero warnings.
- **Files**: `test/unit/server/probe_*.rs`, `test/unit/authorizer/*`, `document/handoff/a9k2_authorizer.md`
- **Status**: pending

### Risks
- Duplicate EntityUid panic if seen set dropped — retained dedup.
- Build drift — authorizer now owns codegen.

### Open questions
- None; snapshot API adopted.

————————————————————————————————————————————————————————————————
