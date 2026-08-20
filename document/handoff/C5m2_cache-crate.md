## Task I: split cache into standalone crate

**Owner**: QkzP7w
**Exec doc**: `document/exec/C5m2_cache-crate.md`
**Status**: Design settled; at adoption gate (no code yet)

### Stage A. Cache crate (slice 1)

- 1. Create `code/cache/` crate (CacheValue/Cache/Caches, value types, validation), workspace member. Status: not started.

### Stage B. Back rewire (slice 2)

- 1. back deps + delete `repository/cache.rs` + config TTL split + `Caches::new` + logic call-site migration + `cache_key` move to `session.rs`. Status: not started.

### Decisions (user-confirmed)

- Crate `cache`, deps `moka` + `std` only (no project crate).
- CRUD methods: `insert`/`read`/`delete`/`delete_if`/`delete_by_reverse_key` (no `consume`; `delete` merged the redundant consume/delete pair).
- Value types validate at construction; table name = value type name = TTL param name.
- Six tables: `user_creation`(EmailAddressHash)/`session`(UserId)/`email_update`(OldAndNewEmailAddressAndTokenHashes)/`user_deletion`(UserIdAndEmailAddressHash)/`challenge`(Challenge)/`download`(VersionIdAndUserId).
- Config: split `token_ttl_seconds` → user_creation/email_update/user_deletion; rename `download_token_ttl_seconds`→`download_ttl_seconds`, `token_cache_capacity`→`cache_capacity`.
- **Open question (at gate)**: UUIDv7 validation — hand-roll format check (std-only) vs add `uuid` dependency. Pending user.
- **Blocking dependency**: hash unification (Task II H7aU) changes email/token hash lengths to a single 128-bit format, enabling `EmailAddressHash`/`TokenHash` to merge into one `Hash` type. Decide whether to proceed with separate lengths now or wait for H7aU.

### Pending confirmation

- Plan adoption at gate (§7): plan, UUID validation choice, red-phase note (pure refactor, existing tests as pin).