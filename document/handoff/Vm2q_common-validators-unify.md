# Handoff

## Task II: Common validators unification

**Owner**: Pq3ZwL
**Exec doc**: `document/exec/Vm2q_common-validators-unify.md`
**Status**: All slices green; awaiting orchestrator review/close.

### Stages

A. ✅ Extract shared skeleton into `code/common/src/validate.rs`
B. ✅ Route `validate_name` / `validate_tag_name` through the shared helper
C. ✅ Route `validate_ascii_text` through the shared helper

### Slices

1. ✅ Add shared module: `CharPolicy`, `ValidationError`, `validate_with_policy`,
   `AlphanumericDashUnderscore`, `PrintableAscii`; `pub mod validate;` +
   `test/unit/common/validate/tests.rs`. Commit `454e4a2`.
2. ✅ Route name + tag; `impl ValidationError for NameError`/`TagNameError`.
   Commit `f030c6f`.
3. ✅ Route text; `impl ValidationError for TextError`. Commit `cb8243c`.

### Decisions made

- Shared `validate.rs` holds a `CharPolicy` trait (per-domain char rule) and a
  `ValidationError` trait (maps `empty`/`too_long(max_chars)`/`forbidden(char)`
  onto each domain's private enum). `validate_with_policy::<E,P>` is the shared
  skeleton (trim → reject blank → scan chars → length cap → return trimmed).
- The three public error enums `NameError`, `TagNameError`, `TextError` and all
  their `Display` strings are preserved byte-for-byte; public fn signatures
  unchanged; `MAX_NAME_CHAR_COUNT` / `MAX_TAG_NAME_CHAR_COUNT` stay as named
  constants fed as `max_chars`.
- Text's two original `ContainsForbiddenChar` branches fold into the combined
  `PrintableAscii` predicate with identical rejection set and reported char.
- Red-phase for behavior-preserving slices 2/3 is the pre-existing unmodified
  per-domain tests; the real red test is slice 1 (orchestrator-approved).
- Concurrent agent's `code/front/...` changes were never staged/committed.

### Code changes

- `code/common/src/validate.rs` (new): shared traits + helper + two policies.
- `code/common/src/lib.rs`: `pub mod validate;`.
- `code/common/src/{name,tag,text}.rs`: wrappers + `impl ValidationError`.
- `test/unit/common/validate/tests.rs` (new): shared-module tests.

### Final gate

- ✅ `cargo fmt --check` — clean (`code/common`)
- ✅ `cargo clippy -- -D warnings` — zero warnings (`code/common`)
- ✅ `cargo test` — 116/116 pass (`code/common`)
- ✅ `cargo +nightly check` — clean in `code/back` and `code/front` (no ripple)

### Open items

- Per handoff readme rule 2, a fully-complete task is removed from the handoff
  and its exec doc deleted once the orchestrator closes it. Pending review, the
  exec doc and this entry are left in place for inspection.

### Questions

- Orchestrator: close/remove this task entry + delete
  `document/exec/Vm2q_common-validators-unify.md` after review?

————————————————————————————————————————————————————————————————