## Task II: unify hashing

**Owner**: RcD9sL
**Exec doc**: `document/exec/H7aU_hash-unify.md`
**Status**: In progress — slice 1 done, slices 2–3 pending

### Decisions (user-confirmed)

- Single hash entry using **AsconCxof128**, salt = value, 128-bit output
  (16 bytes / 32 hex). Deterministic → email lookup unaffected.
- Used by email/token/id hashes; **PDF content hash stays streaming
  AsconXof128** (256-byte customization limit; dedup/filename hash, not
  credential).
- Accepted: "salt = value" is not real salt; token downgrades 256→128 bit;
  migration requires re-seed + re-login.

### Plan (CI-gate aware: `email()`/`token()` stay until back migrates)

- A.1 `hash()` added to common + probe 003 + common tests on `hash()` — **DONE**
- A.2 Migrate production callers (seed.rs, seed_demo.rs, email.rs ×4, cache.rs)
- A.3 Migrate back tests (101 `hash::email` sites / 32 files + committed probes
  probe_001, probe_review_findings), then delete `email()`/`token()`

### Probe outcome (2026-08-20)

- probe_003: salt=value construction deterministic, 32 hex chars, distinct
  inputs → distinct digests, no errors (incl. empty input). Common crate
  103 tests green locally; CI gate per slice.

### Coordination

- Enables Task I (C5m2) to merge `EmailAddressHash`/`TokenHash` into one
  `Hash` type. Decide ordering against Task I.

### Remaining risks

- Back test migration is mechanical but wide (32 files); any missed caller
  breaks CI — gate via ci-watch per slice.
- Runtime DB (`data/`) invalidated → user must re-seed after completion.