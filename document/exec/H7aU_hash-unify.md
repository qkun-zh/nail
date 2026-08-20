# Exec doc — H7aU: unify hashing (Task)

Owner: RcD9sL. Design recorded so the decision is not lost. Separate from
C5m2 (cache crate). No code yet.

## Requirement

Unify `code/common/src/hash.rs` to a single hashing entry used by every
credential/string hash (email, token, ids): Ascon-CXOF128 with **salt = the
value being hashed**, output **128 bits** (16 bytes / 32 hex chars).

```rust
pub fn hash(value: &[u8]) -> anyhow::Result<String> {
    let mut cxof = AsconCxof128::try_new_customized(value)?;  // salt = value
    cxof.update(value);
    let mut output = [0u8; 16];
    cxof.finalize_xof().read(&mut output);
    Ok(hex::encode(output))
}
```

Deterministic (salt derived from value → same value hashes identically), so
`email_address_hash` lookup still works.

## Decisions recorded (from conversation)

- Variant: **`AsconCxof128`** (customizable XOF) — the only variant in
  `ascon-xof128-0.2.1` that accepts a salt (customization). `AsconXof128`
  takes no salt. Evidence: pinned source
  `ascon-xof128-0.2.1/src/{lib,cxof}.rs` — `try_new_customized`, OutputSize
  U32, CollisionResistance U16 (128-bit), extendable output.
- 128-bit output: read 16 bytes.
- "Salt = value" provides no real salt benefit (salt must be unknown to the
  attacker; value is guessable, e.g. low-entropy email) — accepted by user.
  Effective construction is a deterministic 128-bit Ascon hash.
- **Token downgrade**: token hash goes from 256-bit (32 bytes) to 128-bit
  (16 bytes). Accepted by user (short-lived high-entropy tokens).
- **PDF content hash is the exception**: stays streaming `AsconXof128`
  (dedup/filename hash, not a credential). Cannot use CXOF customization —
  `MAX_CUSTOMIZATION_LEN = 256` bytes (cxof.rs:15), PDFs are larger. Accepted.
- **Migration impact**: changing the construction invalidates all stored
  `email_address_hash`, `content_hash`, and every previously issued token hash
  → requires re-seed and re-login. Accepted by user.

## Scope

In scope: `code/common/src/hash.rs`, all callers of `nail_common::hash`
(`email`/`token`/`pdf`), and the affected tests in
`test/unit/common/hash/`.

Out of scope: `code/cache/**` value types (that is task C5m2 — this task
enables unifying them into one `Hash` type afterwards), `code/front/**`,
`Cargo.lock`, `target/`/`dist/`/`data/`/`log/`.

## Confirmed design (from conversation + source)

### Current state (baseline, from `code/common/src/hash.rs`)

| fn | primitive | salt | output | hex len |
|---|---|---|---|---|
| `email(value)` | AsconXof128 | none | 16 B (128 bit) | 32 |
| `token(value)` | AsconCxof128 | `b"token-hash"` (static) | 32 B (256 bit) | 64 |
| `pdf(value)` | AsconXof128 | none | 16 B (128 bit) | 32 |
| `PdfHasher` | AsconXof128 (streaming) | none | 16 B (128 bit) | 32 |

### Target (confirmed)

One entry point for every credential/string hash:

```rust
pub fn hash(value: &[u8]) -> anyhow::Result<String> {
    let mut cxof = AsconCxof128::try_new_customized(value)?;  // salt = value
    cxof.update(value);
    let mut output = [0u8; 16];                                // 128 bit
    cxof.finalize_xof().read(&mut output);
    Ok(hex::encode(output))
}
```

- `email()`/`token()` replaced by `hash()` (callers pass the same bytes).
- `pdf()`/`PdfHasher` stay unchanged (streaming AsconXof128, 16 B / 32 hex) —
  confirmed exception: `MAX_CUSTOMIZATION_LEN = 256` bytes (cxof.rs:15) rules
  out salt=value for MB-sized PDFs, and content hash is a dedup/filename hash,
  not a credential.

### Source evidence (pinned `ascon-xof128-0.2.1`)

- `AsconCxof128` is the only variant accepting a customization/salt
  (`TryCustomizedInit::try_new_customized`, cxof.rs:31-68). `AsconXof128`
  takes none (lib.rs:17-19).
- `OutputSizeUser = U32`, `CollisionResistance = U16` (128-bit), extendable
  output — any digest length readable (cxof.rs:73-80, 90-98).
- Salt maximum 256 bytes (cxof.rs:15).

### Full caller list to migrate (grep evidence)

Production:
- `back/src/repository/seed.rs:18` — `hash::email(user_zero_email)`
- `back/src/repository/seed_demo.rs:157` — `hash::email(&email)`
- `back/src/logic/email.rs:94,122,135,248` — `hash::email(...)`
- `back/src/repository/cache.rs:9` — `hash::token(...)` (dies with Task I,
  moves to `logic/session.rs` as `cache_key`)
- `back/src/interface/multipart.rs:3,38` — `PdfHasher` (UNCHANGED)

Tests:
- `test/unit/common/hash/tests.rs` (token/pdf tests; email-digest assertions
  change length 32→32 hex but value changes)
- `test/unit/back/context.rs:116`, `test/unit/back/http/{comment,multipart,
  content,version}.rs`, `test/unit/back/logic/version.rs` — `hash::pdf`
  (UNCHANGED behavior, no edits needed)

### Accepted trade-offs (user-confirmed)

1. "Salt = value" is not real salt (salt must be unknown to the attacker;
   value is guessable, e.g. low-entropy email). Effective construction is a
   deterministic 128-bit Ascon hash. Accepted.
2. Token hash downgrades 256 → 128 bit. Accepted (short-lived high-entropy
   tokens; 128-bit is sufficient).
3. Migration: every stored `email_address_hash`, every previously issued
   token hash, and seeded data become invalid → re-seed + re-login. Accepted.
4. Determinism preserved → `read_user_by_email_address_hash` lookup works.

### Coordination with Task I (C5m2)

- After this task, `EmailAddressHash` and `TokenHash` (C5m2 crate) become the
  same 128-bit format and can merge into one `Hash` type.
- Task I proceeds meanwhile with the current two lengths (32/64 hex) unless
  the user orders otherwise.

## Slice breakdown

CI gates all crates together, and back still compiles only while `email()`/
`token()` exist → the common change is split so every push stays CI-green:

1. **Add unified `hash()` (common only)** — probe 003 + `hash()` alongside
   transitional `email()`/`token()`; common tests now cover `hash()`.
   Exit: `cargo test -j 1 -p nail_common` green (probe + tests), CI green.
2. **Migrate production callers** — `seed.rs:18`, `seed_demo.rs:157`,
   `logic/email.rs:94,122,135,248`, `repository/cache.rs:9` → `hash()`
   (propagate with `?`; map to `LogicError::internal` in `email.rs`).
   Exit: CI green.
3. **Migrate back tests + delete `email()`/`token()`** — all
   `hash::email`/`hash::token` sites under `test/unit/back/` (incl. committed
   probes `probe_001_read_gate_assembly_baseline.rs`,
   `probe_review_findings.rs` — they break compilation otherwise), then remove
   the transitional fns. Exit: `grep -r 'hash::\(email\|token\)' code test`
   → no matches; CI green.

## Open unknowns

- Exact final construction (salt + message ordering) to confirm against a
  probe test before adoption. → **RESOLVED by probe 003** (2026-08-20):
  construction in "Confirmed design" is deterministic, 128-bit (32 hex),
  distinct across inputs, error-free for normal inputs; all assertions green
  under `cargo test -j 1 -p nail_common`.
- Full list of `nail_common::hash::*` callers to migrate (grep `hash::`).
  → **RESOLVED** (2026-08-20): production 7 sites / 5 files (see Confirmed
  design); back tests 101 `hash::email` sites across 32 files under
  `test/unit/back/` + the two committed probe files; `hash::token` only in
  `repository/cache.rs:9` and common `tests.rs`; no callers in
  front/emailer/pow. `hash::pdf`/`PdfHasher` callers unchanged.

## Verification plan

- Correctness: unified hash unit tests; all common/back tests (re-seeded).
- Behavior change: ALL hash outputs change — expected, the point of the task.
- Data: re-seed + re-login required.

## Risks

- Wide blast radius (every hash in the system). Mitigated by a probe test
  first and full re-seed.
- PDF exception must be documented so "all hashes unified" is not assumed.

## Constraints

- ascon only (README). No `unwrap`/`expect`/new panics. English only.
- No hand-edited `Cargo.lock`; never touch `target/`/`dist/`/`data/`/`log/`.
- Gate is CI, not local builds.

## Questions

1. Orchestrator: approve when ready to start (task currently deferred).
2. Confirm PDF content hash stays `AsconXof128` streaming (not unified).

## Change log

- 2026-08-20: design recorded. Task deferred (separate from C5m2).
- 2026-08-20: added "Confirmed design" section — baseline table, target
  construction, source evidence, full caller list (grep), accepted trade-offs,
  coordination with Task I. Self-contained for any agent.
- 2026-08-20: probe 003 (`test/unit/common/hash/probe_003_salt_equals_value_deterministic.rs`)
  written; RED (compile error, `hash()` missing) → `hash()` implemented →
  GREEN (probe + 103 common tests). Evidence recorded in Open unknowns.
  Slice breakdown refined for the CI-gate constraint: `email()`/`token()`
  stay transitional until all back callers migrate (slice 3).