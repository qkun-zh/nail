# Exec ABCD slice2-text-helpers

## Requirement
Deduplicate helpers as scoped.

## Scope in/out
In: three helpers. Out: other refactors.

## Design
- validate_ascii_text_capped in server/src/logic/error.rs
- usize_capped in same or common
- uuidv7_secs_or_zero in common/src/time.rs

## Slice breakdown
Slice1: create helpers and replace duplicates, one commit.

## Verification
cargo test -p common, cargo fmt, cargo clippy -p common -p server

## Risks
None

## Constraints
No unwrap, no Cargo.lock edit

## Questions
None
