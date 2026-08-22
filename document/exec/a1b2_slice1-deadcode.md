# Exec a1b2 slice1
## Requirement: Remove DeleteBody, inline alias, prune server time macros.
## Scope in: request.rs DeleteBody, session.rs alias, server Cargo macros. Out: other features kept (verified used).
## Design: delete struct/tests for it, replace hash_canonical_token with cache_key.
## Slices: 1) single slice all changes.
## Verification: cargo test -p common, cargo test -p pow --all-targets, fmt, clippy.
## Risks: test removal.
## Constraints: no Cargo.lock manual, no unwrap.
