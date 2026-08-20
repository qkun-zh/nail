# Exec doc — C5m2: split cache into standalone crate (Task)

Owner: QkzP7w. Orchestrator approves at the adoption gate. Single source of
truth for this task.

## Requirement

Extract `code/back/src/repository/cache.rs` into a standalone workspace crate
`code/cache/` (package `cache`) depending on `moka` + `uuid` + `std` — no
project crate dependency. Expose `Caches` (six tables: `user_creation`,
`session`,
`email_update`, `user_deletion`, `challenge`, `download`) and the generic
`Cache<E: CacheValue>` (`new`/`insert`/`read`/`delete`/`delete_if`/
`delete_by_reverse_key`). Keys are opaque `&str`; value types validate at
construction and cannot represent an invalid value. Table name = value type
name = TTL parameter name. The crate owns its configuration file
(`configuration/cache.toml`) and loads it itself (`CacheConfig` + `Caches::load`),
like emailer. Configuration keys: `user_creation_ttl_seconds`/
`session_ttl_seconds`/`email_update_ttl_seconds`/`user_deletion_ttl_seconds`/
`challenge_ttl_seconds`/`download_ttl_seconds`/`cache_capacity` — moved out of
`server.toml`. The `token_key` hashing helper moves to `logic/session.rs`.
Behavior is identical to the current cache — a pure refactor with no behavior
change.

Acceptance criteria:
- `code/cache/` is a standalone crate, compiles independently, and is a
  workspace member; it depends only on `moka` + `uuid` + `std`.
- `back` depends on `cache`; `repository/cache.rs` is deleted; no
  `TokenCaches`/`TokenCache`/`CacheEntry`/`*Entry`/`token_key` remain.
- All callers use the new table names (`user_creation`, `user_deletion`) and
  CRUD methods (`delete`, `delete_if`); no `create_user`/`delete_user`/
  `consume`/`consume_if` remain in back.
- `server.toml` no longer holds cache keys; `configuration/cache.toml` (owned
  by the crate) has seven keys; `back` loads it via `cache::CacheConfig::load`
  and builds `Caches::new(&config.cache)`.
- Zero-warning gate green; back tests pass; no behavior change; English only;
  no `unwrap`/`expect`/new panics.

## Scope

In scope: `code/cache/**`, `code/Cargo.toml`, `code/back/Cargo.toml`,
`code/back/src/repository.rs`, `code/back/src/infrastructure/server.rs`,
`code/back/src/infrastructure/config.rs`,
`code/back/src/infrastructure/config/server.rs`,
`code/back/src/logic/{session,email,user,pow,download,challenge}.rs`,
`configuration/server.toml`.

Out of scope: `code/front/**`, `code/common/**`, `code/emailer/**`,
`code/pow/**`, other back modules, `Cargo.lock`,
`target/`/`dist/`/`data/`/`log/`.

## Design decisions

- Deep module: `Cache<E>` hides the moka `entries` map, the `reverse_index`
  map, and the eviction-listener wiring behind a six-operation interface
  (`insert`/`read`/`delete`/`delete_if`/`delete_by_reverse_key`). Deletion test:
  removing the crate scatters the reverse-index bookkeeping back across the
  callers.
- Value newtypes validate at construction (`EmailAddressHash::new` etc.
  return `Result<_, CacheError>`), so invalid values are unrepresentable and
  every `Cache` method is infallible (no `Result` in the interface). This keeps
  call sites like `cache.challenge.delete(&id).is_none()` unchanged in shape.
- CRUD vocabulary per README §3: `delete` (replacing the now-removed
  redundant `consume`/`delete` pair — both were remove-and-return) and
  `delete_if` (atomic conditional removal) replace `consume`/`consume_if`.
- Literal naming: each table, its value type, and its TTL parameter share one
  noun (e.g. `user_creation` / `Cache<EmailAddressHash>` / `user_creation_ttl`).
- Independence: the crate knows nothing about `nail_common`; hashing lives in
  the caller. Value validation encodes only format facts: 32-hex hash shape
  (manual check) and UUIDv7 shape via `uuid::Uuid::parse_str` (user decision:
  add the third-party `uuid` dependency).
- Keys are opaque `&str`: the crate never hashes; callers pre-hash with
  `cache_key` (= `nail_common::hash::token`) now moved to `logic/session.rs`.

## Confirmed API (final design, from conversation)

### Primitive types (validate at construction)

```rust
pub enum CacheError { InvalidHash, InvalidId }

pub struct Hash(String);             // new(String) -> Result<Self, CacheError>  ascon hex, 32 chars
pub struct UserId(String);           // new(String) -> Result<Self, CacheError>  UUIDv7 via uuid crate
pub struct VersionId(String);        // new(String) -> Result<Self, CacheError>  UUIDv7 via uuid crate
pub struct ChallengeId(String);      // new(String) -> Result<Self, CacheError>  UUIDv7 via uuid crate
pub struct Challenge;                // unit, no validation
```

> H7aU (done) unified hashing to one 128-bit `nail_common::hash::hash()`:
> the former `EmailAddressHash` and `TokenHash` (64-hex token) merged into
> the single `Hash` type above. Field names keep the old spellings
> (`email_address_hash`, `email_token_hash`) to minimize back churn.

### trait + generic cache

```rust
pub trait CacheValue: Clone + Send + Sync + 'static {
    fn reverse_key(&self) -> Option<&str> { None }
    fn validate(&self) -> Result<(), CacheError> { Ok(()) }
}

pub struct Cache<E: CacheValue> {
    entries:       moka::sync::Cache<String, E>,
    reverse_index: moka::sync::Cache<String, Vec<String>>,   // entity -> its batch of keys
}

impl<E: CacheValue> Cache<E> {
    pub fn new(ttl: Duration, capacity: u64) -> Self
    pub fn insert(&self, key: &str, value: E)                  // value validated at construction
    pub fn read(&self, key: &str) -> Option<E>
    pub fn delete(&self, key: &str) -> Option<E>               // remove + return
    pub fn delete_if(&self, key: &str, matches: impl FnOnce(&E) -> bool) -> Option<E>
    pub fn delete_by_reverse_key(&self, reverse_key: &str) -> u64  // bulk invalidate by entity
}
```

### six value types (name = stored content)

```rust
// user_creation: single hash
pub struct Hash(String);                                       // reverse_key = itself

// session: single user id
pub struct UserId(String);                                     // reverse_key = itself

// challenge: empty
pub struct Challenge;                                          // no reverse

// email_update: four hashes
pub struct OldAndNewEmailAddressAndTokenHashes {
    pub old_email_address_hash: Hash,
    pub new_email_address_hash: Hash,
    pub old_email_token_hash:   Hash,
    pub new_email_token_hash:   Hash,
}                                                              // no reverse

// user_deletion
pub struct UserIdAndEmailAddressHash {
    pub user_id: UserId,
    pub email_address_hash: Hash,
}                                                              // reverse_key = user_id

// download
pub struct VersionIdAndUserId {
    pub version_id: VersionId,
    pub user_id: UserId,
}                                                              // no reverse
```

### aggregation

```rust
pub struct Caches {
    pub user_creation: Cache<Hash>,
    pub session:       Cache<UserId>,
    pub email_update:  Cache<OldAndNewEmailAddressAndTokenHashes>,
    pub user_deletion: Cache<UserIdAndEmailAddressHash>,
    pub challenge:     Cache<Challenge>,
    pub download:      Cache<VersionIdAndUserId>,
}

impl Caches {
    pub fn new(
        user_creation_ttl: Duration,
        session_ttl: Duration,
        email_update_ttl: Duration,
        user_deletion_ttl: Duration,
        challenge_ttl: Duration,
        download_ttl: Duration,
        capacity: u64,
    ) -> Self
}
```

### table overview

| table | main key | value type | reverse entity | TTL param |
|---|---|---|---|---|
| `user_creation` | email token hash | `Hash` | itself | `user_creation_ttl` |
| `session` | session token hash | `UserId` | itself | `session_ttl` |
| `email_update` | `UserId` | `OldAndNewEmailAddressAndTokenHashes` | none | `email_update_ttl` |
| `user_deletion` | delete token hash | `UserIdAndEmailAddressHash` | `UserId` | `user_deletion_ttl` |
| `challenge` | `ChallengeId` | `Challenge` | none | `challenge_ttl` |
| `download` | download token hash | `VersionIdAndUserId` | none | `download_ttl` |

### hash-length facts (H7aU unified, done 2026-08-20)

- `nail_common::hash::hash()` → AsconCxof128 (salt = value), 16 bytes →
  **32 hex** for every credential/string hash (email, token, ids). Source:
  `code/common/src/hash.rs` (read). Every `Hash` validates 32 hex.

## Slice breakdown

1. **cache crate** (new code, no callers). Files: `code/cache/Cargo.toml`,
   `code/cache/src/lib.rs`, `code/cache/src/cache.rs`, `code/cache/src/value.rs`,
   `code/Cargo.toml` (add member). Red: none (new crate). Green: crate compiles
   standalone; unit tests for `Cache` operations + value validation pass.
   Exit: CI builds `cache`.
2. **back rewire** (atomic — logic depends on the old types). Files:
   `back/Cargo.toml` (add `cache`), `back/src/repository.rs` (drop `cache`),
   delete `back/src/repository/cache.rs`,
   `back/src/infrastructure/config.rs` + `config/server.rs` (TTL split +
   renames), `back/src/infrastructure/server.rs` (`Caches::new` with six TTLs),
   `back/src/logic/{session,email,user,pow,download,challenge}.rs` (new table
   names + `delete`/`delete_if` + new value types; `cache_key` moved to
   `session.rs`), `configuration/server.toml` (six TTLs + `cache_capacity`).
   Red: none (refactor; existing back tests are the regression pin). Green:
   back compiles and all back tests pass. Exit: CI back tests green.

## Open unknowns

- RESOLVED (2026-08-20, user): UUIDv7 validation via the third-party `uuid`
  crate — `uuid::Uuid::parse_str` in the value constructors (new dep of
  `cache`). Evidence: `back` already generates ids with `uuid::Uuid::now_v7()`.
- RESOLVED (H7aU done): unified `hash()` → 128-bit → 32 hex for every
  credential hash; `EmailAddressHash`/`TokenHash` merged into `Hash`.
- moka `Cache::builder`/`entry`/`and_compute_with` API — already exercised by
  the existing `cache.rs`; source available in pinned registry, no standalone
  probe needed.

## Verification plan

- Correctness: existing unmodified back tests run as the regression pin each
  slice; new `cache` unit tests cover operations and value validation.
  Verified via CI.
- Behavior change: none (refactor); proven by unmodified tests staying green.
- Time/space complexity: unchanged (same moka operations, same data).
- Performance: unchanged (same moka cache semantics, same reverse-index
  bookkeeping).

## Risks

- UUIDv7 validation via the `uuid` crate is authoritative (parse + version
  check) — risk of false accept/reject removed; dedicated unit tests still
  cover the newtype constructors.
- Large atomic rewire slice risks a big diff — accepted; necessary for
  compilation; CI catches breakage.
- Value-format coupling makes the crate encode ascon/UUID facts — accepted for
  a project-specific crate; flagged in Design decisions.

## Constraints

- `cache` depends on `moka` + `uuid` + `std`; no project crate dependency.
- No `unwrap`/`expect`/new panics; no comments restating code; English only.
- No hand-edited `Cargo.lock`; never touch `target/`/`dist/`/`data/`/`log/`.
- Check machine load before any build; back off if loaded. The gate is CI, not
  local — no local `cargo test` unless required for a red-phase.
- One commit per slice, clean tree; never discard work.

## Questions

1. Orchestrator: approve this plan at the adoption gate?
2. UUIDv7 validation: RESOLVED (user, 2026-08-20) — add the third-party
   `uuid` crate.
3. Accept the documented red-phase note (refactor; existing back tests are the
   regression pin, no genuinely-red test)?

## Change log

- 2026-08-20: initial plan. Design settled in conversation before this doc.
- 2026-08-20: added "Confirmed API" section — full final design (primitive
  types, trait, `Cache` methods, six value types, `Caches` + `new` signature,
  table overview, hash-length facts) so the exec doc is self-contained for any
  agent. Open gate questions (UUID validation, plan adoption) unchanged.
- 2026-08-20: user decision — UUIDv7 validation via `uuid` crate (new dep);
  `EmailAddressHash`/`TokenHash` merged into single `Hash` (H7aU done, 32 hex);
  docs updated everywhere. Remaining gates: plan adoption + red-phase note.
- 2026-08-20: slice 1+2 DONE by subagent (commits 2c36b00..fe565df, CI
  #32390381683 green): crate + back rewire. 
- 2026-08-21: slice 3 DONE — user decision: crate owns its own config file
  (`configuration/cache.toml`) and reads it itself like emailer
  (`CacheConfig::load`/`validate` + `Caches::load`; `Caches::new(&CacheConfig)`).
  Back drops the seven server.toml keys and accessors; `Configurator` reads
  `download_token_ttl_seconds` from the cache config. Commits d2623ff, 57eaee0,
  62ebfd5, 1ff8fc4; CI #32427406154 green. TASK COMPLETE.
- 2026-08-20: slice 1 DONE — `code/cache/` committed (`2c36b00`, `baf031b`,
  `5e73f02`); CI run #32385777082 green. `code/cache/Cargo.lock` generated
  standalone in `/tmp/cachelockgen` (empty `[workspace]` table; version-4 lock
  has no `root` key). CI toolchain is 1.97 vs local 1.96: `baf031b` missed
  `test/unit/cache/tests.rs` (staged only `code/cache`) — fmt/clippy 1.97
  failures; `5e73f02` wrapped the ChallengeId `assert_eq!` and switched
  `from_secs(300/60)` → `from_mins(5/1)` (new `clippy::duration_suboptimal_units`).
- 2026-08-20: slice 2 DONE — back rewire committed (`8d5d025`, then fixes
  `c8a2852` version_id format string / `1ee18fe` URL built before the move /
  `6cbbb27` must_use discards / `fe565df` probe test real UUIDv7); CI run
  #32390381683 green. Three CI iterations failed before green: (1) E0277
  `cache::VersionId` lacks Display + E0609 `entry.email_address_hash` on
  `Hash`; (2) E0382 moved `version_id` borrowed in format; (3) unused
  must_use results of `delete`/`delete_if`/`delete_by_reverse_key` (16 sites,
  fixed with `let _ =`); (4) `probe_review_findings.rs:63` used "user-123"
  which `UserId::new` rejects — replaced with `Uuid::now_v7()`.
- 2026-08-20: slice 3 DONE — leftover grep clean within scope (remaining
  `download_token_ttl_seconds` matches are the intentional out-of-scope wire
  contract: `interface/config.rs:20`, `infrastructure/state.rs:38`,
  `test/unit/back/http/config.rs:20`); handoff + this Change log updated;
  docs commit `[skip ci]`. Task COMPLETE — exec doc retained per repo practice
  (workflow §9 deletion deferred until orchestrator closes the task).