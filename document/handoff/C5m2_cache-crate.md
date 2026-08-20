## Task I: split cache into standalone crate

**Owner**: QkzP7w
**Exec doc**: `document/exec/C5m2_cache-crate.md`
**Status**: COMPLETE — all 3 slices committed, CI green; handoff pending orchestrator final gate review

### Stage A. Cache crate (slice 1) — DONE

- 1. Create `code/cache/` crate (CacheValue/Cache/Caches, value types, validation), workspace member. Status: DONE — `2c36b00` (feat), `baf031b` (fix: must_use/doc_markdown, add code/cache/Cargo.lock generated standalone), `5e73f02` (test fix: ChallengeId assert wrap + duration units). CI run #32385777082 green.
- Deviation: `ci.yml` gained the cache crate's fmt/clippy/test/audit steps (via `Swatinem/rust-cache` cache-key bump in `d201055`; cache steps added by this task).

### Stage B. Back rewire (slice 2) — DONE

- 1. back deps + delete `repository/cache.rs` + config TTL split + `Caches::new` + logic call-site migration + `cache_key` move to `session.rs`. Status: DONE — `8d5d025` (rewire, 30 files), `c8a2852` (fix: version_id format string + user_creation entry value), `1ee18fe` (fix: build URL before moving version_id), `6cbbb27` (fix: discard must_use delete results), `fe565df` (fix: probe test needs a real UUIDv7). CI run #32390381683 green (fmt/clippy/test/audit/frontend all pass).
- No behavior change: `download_token_ttl_seconds` wire field (nail_common RuntimeLimits + interface/config.rs + http/config.rs) intentionally unchanged.

### Stage C. Docs + report (slice 3) — DONE

- 1. Handoff + exec doc Change log. Status: DONE — docs commit `[skip ci]`.

### Decisions (user-confirmed)

- Crate `cache`, deps `moka` + `uuid` + `std` (no project crate).
- CRUD methods: `insert`/`read`/`delete`/`delete_if`/`delete_by_reverse_key` (no `consume`; `delete` merged the redundant consume/delete pair).
- Value types validate at construction (`uuid::Uuid::parse_str` for UUIDv7 ids — **user chose the `uuid` dependency**); table name = value type name = TTL param name.
- Six tables: `user_creation`(Hash)/`session`(UserId)/`email_update`(OldAndNewEmailAddressAndTokenHashes)/`user_deletion`(UserIdAndEmailAddressHash)/`challenge`(Challenge)/`download`(VersionIdAndUserId).
- Config: split `token_ttl_seconds` → user_creation/email_update/user_deletion; rename `download_token_ttl_seconds`→`download_ttl_seconds`, `token_cache_capacity`→`cache_capacity`.
- **H7aU done (2026-08-20)**: unified `hash()` → 128-bit → 32 hex; `EmailAddressHash`/`TokenHash` merged into single `Hash` (32 hex). Field names keep old spellings.