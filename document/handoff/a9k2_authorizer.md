## Task I: authorizer standalone crate

**Owner**: x7K2pQ
**Exec doc**: `document/exec/a9k2_authorizer.md`
**Status**: Slice 1 done, Slice 2 done, Slice 3 done — ready for final gate

### Stage A: Crate extraction (searcher parity)

#### Slice 1: crate skeleton
- **Goal**: standalone `authorizer` crate validates policy/schema and authorizes via snapshot.
- **Files**: `code/authorizer/*`, `code/Cargo.toml`, `test/unit/authorizer/*`
- **Status**: done — `cargo test -p authorizer` 5 passed, `clippy` clean, `fmt` clean
- **Decisions**: API `Principal/Resource` snapshot-based; build.rs codegen permissions/entities; Cedars hidden

#### Slice 2: server wiring
- **Goal**: server uses new crate via DB→snapshot adapter, behavior unchanged.
- **Files**: `code/server/src/infrastructure/authorizer.rs`, `code/server/Cargo.toml`, `code/authorizer/src/authorizer.rs`
- **Status**: done — `cargo test -p server` 593 passed, duplicate-entity fix applied, `clippy` clean
- **Confirmations**: snapshot API adopted; duplicate User entity dedup via merge_entities

#### Slice 3: cleanup
- **Goal**: remove legacy baggage, promote probes, zero warnings.
- **Files**: `code/server/src/infrastructure/authorizer.rs`, `code/server/src/infrastructure/cedar.rs`, `code/server/src/repository/authorization.rs`, `code/server/src/logic/error.rs`, `test/unit/server/probe_*.rs`
- **Status**: done — removed AssemblyError From impl in authorizer wrapper, dedup probes 005/006, per-item allow in cedar.rs, 586 server tests green

### Risks
- Duplicate EntityUid panic if seen set dropped — retained dedup.
- Build drift — authorizer now owns codegen.

### Open questions
- None; snapshot API adopted.

————————————————————————————————————————————————————————————————
