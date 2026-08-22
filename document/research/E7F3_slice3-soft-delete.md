# Research: Slice3 soft-delete dedup

## Requirement R0
Deduplicate soft-delete/restore boilerplate in `logic/article.rs`, `logic/version.rs`, `logic/comment.rs` without behavior change. Create `logic/delete.rs` with `soft_delete_guard`/`undelete_guard` returning Result<(),LogicError> encapsulating authorize+is_soft_deleted check. Keep error strings identical ("already soft-deleted"/"not soft-deleted"), same status codes, sync calls, DB effects. Version extra refresh logic stays outside helper.

## Research Questions
1. Exact redundancy boundaries and differences?
2. Helper signature that preserves behavior?
3. No behavior change provable?

## Evidence

### Q1 source
- `article.rs:180-199` Soft: authorize soft → is_soft_deleted(Article) → bad_request "already soft-deleted" → soft_delete_article → sync
- `article.rs:207-228` undelete: authorize undelete → is_soft_deleted → !hidden → "not soft-deleted" → clear_flag → sync
- `version.rs:220-246` Soft: authorize → parent_article_of → is_soft_deleted(Version) → soft_delete_version → refresh_live_latest_version(parent) → sync parent
- `version.rs:271-295` undelete: authorize → is_soft_deleted → clear_flag → parent lookup → refresh → sync
- `comment.rs:254-270` Soft + `283-303` undelete: authorize → is_soft_deleted(Comment) → soft/clear → sync_for_comment
- Reuse baseline 8e5eb0b (green, task says not to re-run full baseline).

### Q1 probe
Probe `test/unit/server/probe_001_soft_delete_guards.rs` verifies helper error paths: soft already-deleted → 400 "already soft-deleted", undelete not-deleted → 400 "not soft-deleted".

### Q2 source
Helper needs `state, actor_id, entity_ref, permission, kind, id` for guard; or generic `do_soft_delete` with closure. Minimal is `soft_delete_guard`/`undelete_guard` as spec suggests. Version needs parent lookup & refresh outside helper via post-hook.

### Q3 source
No logic/user.rs change; error strings preserved.

## Findings
Redundancy confirmed Aug22. Helper can encapsulate authorize+check; version's extra steps remain outside.

## Impact on R
R0 stands, no revision.

## Open items
None.

## Verification
- Correctness: probe helper error paths
- Behavior change: zero delta (same errors/sync)
- Complexity: same Big-O, one less DB roundtrip? no
- Performance: negligible
