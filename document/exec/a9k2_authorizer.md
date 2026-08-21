# Exec: authorizer standalone crate (searcher parity)

## 1. Requirement R (adopted from R₀)
Extract `Authorizer` + Cedar assembly into standalone crate `authorizer` mirroring `searcher` elegance: no `Database` dep, no `anyhow`, no `unwrap`/`expect`, engine hidden, typed `Error::{Denied,NotFound,Internal}`, policy/schema validated once at `Authorizer::new()`, behavior identical to baseline (Cedar decisions, NotFound vs Denied mapping, 39 actions). Server becomes thin DB→snapshot adapter; `logic/authorize` facade preserved.

Acceptance: `cargo test -j 1 -p authorizer` and `-p server` green, `cargo clippy` zero warnings, `cargo fmt` clean, authorizer depends only on `cedar-policy`.

## 2. Scope
In: new crate `code/authorizer` (lib, cedar files, build.rs, error, principal, resource, authorizer), `code/Cargo.toml` workspace member, `code/server` adapter (state, repository/authorization shim or deprecation, logic/authorize forwarding), wiring `searcher`-style re-exports, tests in `test/unit/authorizer`.
Out: policy.cedar/schema.cedar semantics change, permission set change, DB schema change, frontend.

## 3. Design decisions
- Mirror `searcher/src/lib.rs:1-31`: crate-level doc invariants, `pub mod` no engine leak, `pub use` minimal surface, `[cfg(test)] #[path=...]` harness.
- Mirror `searcher/src/error.rs`: typed `Error` enum, `Display`, `source`, `From<io>` not needed; `Internal(String)` only.
- Mirror `searcher/src/schema.rs`: authorizer owns `policy.cedar`+`schema.cedar` via `include_str!`, `Validator::Strict` once in `Authorizer::new()`, `Arc<PolicySet>` shared, no rebuild/marker needed (policy static).
- Snapshot-based API (hexagonal cut like searcher owns IndexDoc, server owns graph→doc adapter):
  ```rust
  pub struct Principal { pub id: String, pub roles: Vec<Role> }
  pub struct Role { pub name: String, pub perms: Vec<String> }
  pub enum Resource { Article{ id:String, owner:String }, Version{ id:String, article_id:String, owner:String }, Comment{ id:String, version_id:String, article_id:String, article_owner:String, owner:String }, Role{ name:String, perms:Vec<String> }, User(String), Tag(String), Virtual(String) }
  pub enum Error { Denied, NotFound, Internal(String) }
  impl Authorizer { pub fn new() -> Result<Self, Error>; pub fn authorize(&self, principal: &Principal, action: &str, resource: &Resource) -> Result<(), Error> }
  ```
  Server adapter `authorizer::adapter::assemble_from_db` is not in authorizer crate; server builds `Principal`/`Resource` from `Database`.
- Single `uid()` helper (like `searcher`'s `to_document`) collapses scattered `format!("{}::\"{}\"")`; dedup HashSet `seen` kept but isolated in `principal::build` and `resource::build`.
- Build.rs codegen for `PERMISSION_*` / `CEDAR_ENTITY_*` moves from `server/build.rs` to `authorizer/build.rs`; server re-exports from authorizer to avoid drift.
- Performance: per-authorize alloc = O(R+P+chain) ≤ ~45 entities; Arc clone only; no DB clone.

## 4. Slice breakdown
Slice 1 — crate skeleton (authorizer crate with policy/schema, error, principal, resource, authorizer core, build.rs, harness; server untouched):
  Goal: standalone authorizer compiles and Cedar tests pass.
  Files: `code/authorizer/Cargo.toml`, `code/authorizer/build.rs`, `code/authorizer/cedar/*`, `code/authorizer/src/lib.rs`, `src/error.rs`, `src/authorizer.rs`, `src/principal.rs`, `src/resource.rs`, `code/Cargo.toml` (add member)
  Red: `cargo test -p authorizer` fails (crate missing)
  Green: `cargo test -j 1 -p authorizer` passes (decision parity with probe_007); `cargo clippy -p authorizer` zero warnings
  Exit: `cargo test -j 1 -p authorizer && cargo fmt --check && cargo clippy -p authorizer`

Slice 2 — server wiring (state::AppState uses authorizer::Authorizer; repository::authorization becomes adapter to snapshot or deprecated shim; logic::authorize forwards; build.rs re-export):
  Goal: server behavior unchanged via snapshot adapter.
  Files: `code/server/src/infrastructure/state.rs`, `code/server/src/repository/authorization.rs` (shim), `code/server/src/logic/authorize.rs`, `code/server/src/infrastructure/authorizer.rs` (removed or re-export), `code/server/build.rs`, `code/server/Cargo.toml` (dep authorizer)
  Red: `cargo test -j 1 -p server probe_007_authorize_orchestration` fails (still on old Authorizer with DB)
  Green: same tests pass via new crate; full `cargo test -j 1 -p server` 570 tests green
  Exit: `cargo test -j 1 -p server && cargo test -j 1 -p authorizer && cargo clippy`

Slice 3 — cleanup & docs (remove legacy baggage: anyhow, duplicate action_uid, scattered UID formats; promote/demerge probes 003-009; handoff):
  Goal: no legacy code remains; docs/harness tidy.
  Files: `test/unit/server/probe_00*.rs` (promote or delete), `test/unit/authorizer/*`, `document/handoff/a9k2_authorizer.md`, `code/server/src/infrastructure/cedar.rs` (re-export)
  Red: `rg -g '!target' authorizer.*Database` still hits or `rg anyhow` in authorizer crate
  Green: no hits; `cargo clippy` zero; research+exec docs ready to delete on gate-final
  Exit: `cargo clippy -- -D warnings && cargo fmt`

## 5. Open unknowns
- None blocking; Cedar Entity::new attr error path verified (probe_009); Version chain parent resolution via DB adapter trusted (probe_006); performance baseline captured in probe_001 (member 3.2ms, admin 23ms) — snapshot build must stay within 1.2×.

## 6. Verification plan
- Dimensions per workflow §4: Correctness (probe_007 decisions), Behavior change delta = none (same NotFound/Denied mapping checked in probe_006/008), Complexity O(R+P+3), Performance probe_001 baseline vs post-slice 2 diff <20%.
- Each slice gate: `cargo fmt; cargo clippy` (zero warnings); `cargo test -j 1 -p <crate>`; push → CI watch `document/ci-watch.sh --once`.
- Final gate: full `cargo test -j 1 -p {server,common,authorizer,searcher,emailer,cache,database}` then `cargo test -j 1 -p pow --all-targets`.

## 7. Risks
- Duplicate EntityUid after snapshot dup (seen set must be kept) → panic in `Entities::from_entities` (probe_005 showed Duplicate). Mitigated by retaining dedup.
- Build.rs drift between crates → divergent PERMISSION_* constants. Mitigated by moving generation to authorizer and re-exporting.
- Comment chain 3-entity parents wrong → owner bypass fails. Mitigated by probe_006/007.

## 8. Constraints
- No `unwrap`/`expect` in `code/authorizer/src` (tests exempt).
- No `anyhow` in authorizer; only `Error`.
- No `Database` dependency in authorizer crate.
- Preserve 39-action vocabulary; policy/schema files duplicated via `include_str!` not `include!` of server path to keep crate standalone (copy, not symlink).
- One commit per slice; clean tree.

## 9. Questions for user (gate-adopt)
1. Adopt snapshot API above vs `AuthRequest` struct wrapper?
2. Keep `repository::authorization::Resource(String)` shim for one release or remove immediately?

