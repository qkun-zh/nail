# Research a1b2 slice1-deadcode
## Requirement R0: Remove DeleteBody, inline hash_canonical_token->cache_key, prune unused cargo features, no behavior change.
## Questions
1. DeleteBody unused? 2. hash_canonical_token alias? 3. Cargo features unused?
## Evidence
- DeleteBody: source common/src/request.rs:32 defines struct; probe grep shows only request_tests.rs uses it, no prod code.
- hash_canonical_token: source server/src/logic/session.rs:20 alias to cache_key; callers in email.rs 5 uses; cache_key identical.
- anyhow: common/src/time.rs uses anyhow::Result -> needed, keep.
- time formatting: common/src/time.rs uses time crate OffsetDateTime formatting -> formatting needed, keep.
- server time macros: grep shows no macro usage -> removable.
- tokio-util: server/src/interface/content.rs uses ReaderStream -> needed, keep.
## Findings: DeleteBody dead in prod; alias redundant; only server time macros is removable.
## Impact: Revise R to keep needed features.
## Open: none
