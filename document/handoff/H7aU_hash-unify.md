## Task II: unify hashing

**Owner**: RcD9sL
**Exec doc**: `document/exec/H7aU_hash-unify.md`
**Status**: DONE (2026-08-20) — commits 1286ee8 (slice 1), 3cfb981 (slice 2),
bc602b6 (length fix). CI green: runs #32372089821, #32381784765.

### Decisions (user-confirmed)

- Single hash entry using **AsconCxof128**, salt = value, 128-bit output
  (16 bytes / 32 hex). Deterministic → email lookup unaffected.
- Used by email/token/id hashes; **PDF content hash stays streaming
  AsconXof128** (256-byte customization limit; dedup/filename hash, not
  credential).
- Accepted: "salt = value" is not real salt; token downgrades 256→128 bit;
  migration requires re-seed + re-login.

### Done

- `hash()` added to common; `email()`/`token()` deleted; PdfHasher untouched.
- Production callers migrated: seed.rs, seed_demo.rs, email.rs ×4
  (`LogicError::internal("failed to hash email: {error}")`), cache.rs
  `token_key`.
- All back tests migrated (~110 sites, 32 files, `.expect("hash must succeed")`
  — tests only, exempt from the no-expect rule via clippy config).
- probe_003 deleted (its acceptance questions are covered by tests.rs).
- One CI fix: `token_key_is_the_ascon_hash_of_the_token` expected 64-hex;
  now 32.

### Outstanding (user action)

- **Re-seed `data/`**: user-zero + demo user email hashes changed; existing
  sessions/challenges invalidated.

### Probe outcome (2026-08-20)

- probe_003: salt=value construction deterministic, 32 hex chars, distinct
  inputs → distinct digests, no errors (incl. empty input). Common crate
  103 tests green locally; CI gate per slice.

### Coordination

- Enables Task I (C5m2) to merge `EmailAddressHash`/`TokenHash` into one
  `Hash` type (32 hex).

### Remaining risks

- Runtime DB (`data/`) invalidated → user must re-seed (action above).