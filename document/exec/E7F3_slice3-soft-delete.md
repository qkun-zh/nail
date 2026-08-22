# Exec: Slice3 soft-delete dedup

## Requirement
Deduplicate soft-delete/restore boilerplate without behavior change.

## Scope
In: logic/delete.rs new helper, refactor article/comment/version to use it. Out: logic/user.rs, error strings/status/sync unchanged.

## Design
`logic/delete.rs` exposes `soft_delete_guard(state, actor_id, entity, permission, kind, id)` and `undelete_guard(...)` -> Result<(),LogicError>. Refactor call sites; version keeps refresh_live_latest_version + sync outside.

## Slice breakdown
1. Create delete.rs helper + refactor 3 files → green, one commit.

## Open unknowns
None.

## Verification
- probe_001 passes
- cargo clippy -p server, cargo fmt, cargo test -j1 -p server

## Risks
Error message drift → keep literal strings.

## Constraints
No behavior change, no Cargo.lock, no unwrap, English docs.

## Questions
None.
