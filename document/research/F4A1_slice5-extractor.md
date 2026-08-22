# Research F4A1 slice5 extractor

## Requirement
R0: Deduplicate AppJson/AppQuery/AppPath FromRequest impls via macro `define_extractor!(AppJson, Json, "invalid request body")` etc, net lines decrease, error messages identical, leave AppMultipart as is.

## Research questions
1. Current impl differences?
2. Error message exact strings?
3. Line count delta feasible?

## Evidence
- Source: `code/server/src/interface/extractor.rs:1-58` read — three structs differ only in trait (FromRequest vs FromRequestParts), axum type (Json/Query/Path), and message.
- Probe: baseline 79835f2 tests green (prior CI). No new behavior change, macro expands to same impl.

## Findings
- AppJson uses FromRequest, others FromRequestParts; macro must handle both or use two variants; simplest: macro handles FromRequestParts vs FromRequest via separate arms or single with tt.
- Messages: "invalid request body", "invalid query parameters", "invalid path parameters" — keep identical.
- Line count: 3 impls ~30 lines -> macro ~15 lines + 3 invocations = net decrease.

## Impact on R
No change.

## Open items
None.
