# Exec F4A1 slice5 extractor

## Requirement
Deduplicate AppJson/AppQuery/AppPath via macro, net lines decrease, identical error messages.

## Scope
In: `code/server/src/interface/extractor.rs`
Out: `interface/multipart.rs`, AppMultipart wrapper

## Design decisions
Macro `define_extractor!` with two arms: FromRequest (Json) and FromRequestParts (Query/Path). Keep DeserializeOwned bounds identical.

## Slice breakdown
- Slice1: Replace three impls with macro, verify net lines decrease, cargo fmt/clippy/test.

Files: `code/server/src/interface/extractor.rs`
Red: N/A (refactor, no behavior change)
Green: existing tests pass
Exit test: `cargo test -j 1 -p server` + clippy

## Open unknowns
None.

## Verification plan
- wc -l before/after, ensure decrease
- cargo fmt, cargo clippy -p server, cargo test -j 1 -p server

## Risks
Macro hygiene, trait mismatch.

## Constraints
No Cargo.lock, English, one commit push.

## Questions
None.
