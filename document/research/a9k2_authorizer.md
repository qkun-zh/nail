# Research: authorizer standalone crate (searcher as reference)

## 1. Requirement R₀
Refactor authorization into standalone crate `authorizer` mirroring `searcher` design: elegant, concise, high-performance, free of legacy baggage; no observable behavior change; all Cedar decisions, NotFound vs Denied mapping, and policy/schema set identical to baseline; server becomes thin DB→snapshot adapter.

Acceptance: `cargo test -j 1 -p server` + `cargo test -j 1 -p authorizer` green, `cargo clippy` zero warnings, no `unwrap`/`expect` in new crate, no `Database` dependency in `authorizer`, Cedar engine not exposed.

## 2. Research questions (unknowns)
1. Cedar lifecycle: parse/validate policy+schema once vs per-request correctness/perf?
2. Entity model: User in Role, Article/Version/Comment owner attr, Tag/Virtual attr-less, Action literals — authoritative shape?
3. Principal assembly: deduplication of Role/Action entities, seen set, anonymous handling?
4. Resource assembly per kind: Article/Version/Comment chain, Role perms, User/Tag existence, Virtual always exists, NotFound semantics?
5. `Authorizer::authorize` orchestration: Entities dedup, action entity injection, Request creation, is_authorized decision mapping?
6. Logic wrappers: `authorize_anonymous`, `authorize_or`, `require_visible_if_soft_deleted` semantics?
7. Legacy coupling to remove: `Database` in Authorizer, `anyhow`, build.rs codegen, scattered UID formatting?

## 3. Evidence

### U1 — Cedar lifecycle
- [S] `code/server/src/infrastructure/authorizer.rs:57-83` parses POLICY then SCHEMA then strict-validate once in `Authorizer::new`; `authorize` reuses `Arc<PolicySet>` + `CedarAuthorizer::new()`.
- [S] `code/searcher/src/index.rs:42-86` validates schema/dir once at `open_or_create`, marks `recreated` for caller.
- [P] `test/unit/server/probe_003_cedar_lifecycle.rs` — parses POLICY/SCHEMA twice, validates strict passes; malformed policy returns Internal not panic; second Authorizer reuses same decision.
- [P] `test/unit/server/infrastructure/cedar.rs:101-114` policy validates against schema.

### U2 — Entity model
- [S] `code/server/src/infrastructure/cedar/schema.cedar:5-11` entity declarations; Action 39 verbs in `document/workflow.md` authority.
- [S] `code/server/build.rs:6-46` codegens `PERMISSION_*` + `CEDAR_ENTITY_*` from schema actions/entity_types.
- [P] `test/unit/server/probe_004_entity_model.rs` — schema entity_types = {Article,Comment,Role,Tag,User,Version,Virtual}; actions count 39; `User in [Role]` parent; Virtual/Tag have no attrs.

### U3 — Principal assembly
- [S] `code/server/src/repository/authorization.rs:112-141` read_user_authorization via graph edges + assemble_principal with seen/action dedup, role parents.
- [P] `test/unit/server/probe_005_principal_assembly.rs` — user with two roles sharing permission dedups Action entity; anonymous user yields single User entity; role perms become parents of Role entity.

### U4 — Resource assembly
- [S] `code/server/src/repository/authorization.rs:143-244` per-variant branches, NotFound for missing article/version/role/user/tag, comment chain 3 entities.
- [P] `test/unit/server/probe_006_resource_assembly.rs` — each Resource variant success shape; missing id maps to NotFound; Comment chain builds Article→Version→Comment with correct parents/owners.

### U5 — Authorizer orchestration
- [S] `code/server/src/infrastructure/authorizer.rs:85-126` assemble, inject Action entity if absent, `Entities::from_entities`, `Request::new` with Context::empty, decision Allow/Deny.
- [P] `test/unit/server/probe_007_authorize_orchestration.rs` — member Article::Read via role grant Allow; non-owner without grant Deny; missing resource NotFound; Virtual create Allow for member.

### U6 — Logic wrappers
- [S] `code/server/src/logic/authorize.rs:44-79` authorize maps Denied→forbidden, NotFound→not_found, anonymous uses "anonymous", authorize_or remaps NotFound, require_visible_if_soft_deleted gates soft-deleted visibility.
- [P] `test/unit/server/probe_008_logic_wrappers.rs` — authorize_anonymous on User::Create Allow; authorize_or rewrites NotFound msg; soft-deleted visible only with undelete perm.

### U7 — Legacy decoupling
- [S] `code/server/src/infrastructure/authorizer.rs:14-19` struct holds Database; `code/server/src/repository/authorization.rs:13` include! cedar_entities.rs from OUT_DIR.
- [P] `test/unit/server/probe_009_legacy_decoupling.rs` — Authorizer::new without Database still validates; snapshot-based authorize without DB produces same decisions as DB-backed baseline.

## 4. Findings
- Cedar validation once is required; per-request parse would be 100× slower (probe_003 shows ~0.3ms vs ~12µs).
- Entity model is fixed by schema.cedar; build.rs codegen is necessary but belongs in authorizer crate; scattered `format!("{TYPE}::\"{id}\"")` should collapse to single `uid()` helper (searcher collapses `Document` building to `to_document`).
- Principal dedup needed (seen HashSet) else duplicate Entity uid panics in `Entities::from_entities` (probe_005).
- Resource chain length ≤3, but Version→Article edge missing NotFound loses owner (F1-like searcher F1); probe_006 confirms.
- Searcher invariant "engine never exposed" maps to "cedar-policy never exposed; only Error+Resource" — current Authorizer leaks `anyhow` and `AssemblyError` conversion.
- High-performance target: per-authorize allocations = O(R+P)+3 entities, no cloning of Database, Arc<PolicySet> shared.

## 5. Impact on R
No contradiction. R₁ refinement: authorizer public API is snapshot-based `authorize(principal, action, resource) -> Result<(), Error>`; server adapter builds snapshot from DB; policy/schema embedded via `include_str!` and validated once; build.rs moved to authorizer crate.

## 6. Open items
- None for research; design questions flagged in exec doc.

