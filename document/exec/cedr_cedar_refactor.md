# Exec Doc — Cedar Conformance Refactor (cedr)

## 1. Requirement
`R₁`: Replace all off-spec Cedar mechanics with documented ones while
preserving every pinned behavior. Acceptance: all pre-existing tests pass;
only intentional delta = malformed authorization requests → 400.
Adoption rulings recorded in `document/research/cedr_proposal.md` §0.

## 2. Scope
In: authorizer crate policy/engine mechanics, schema truthfulness, request +
entity validation, grant projection (DB → template links), reload wiring,
error mapping, tests. Out: HTTP surface, client, search internals, storage,
action-EID renames, role-name reservation (declined by user).

## 3. Design decisions
See proposal §2 D1–D9 and §11 implementation record. Key amendments found
during execution: schema appliesTo widening for four collection gates (F12);
restored uid-dedup merge for self-referential requests; `resource has owner`
defensive guard per docs' defensive pattern.

## 4. Slice breakdown (executed)
1. S1 authorizer core rewrite — DONE
   Files: cedar/schema.cedar, cedar/policy.cedar, src/{authorizer,error,
   principal,resource,lib}.rs, tests/{policy,schema}.rs (kept), new unit tests.
   Red: old fabrication API compile-fails. Green: 24/24 crate tests incl.
   forbids, dynamic reload, loud malformed-request rejection.
2. S2 server integration — DONE
   Files: repository/authorization.rs (+read_all_role_grants), infrastructure/
   authorizer.rs (grants at boot, reload(), BadRequest mapping), logic/
   authorize.rs (400 mapping), logic/role.rs (reload after grant/revoke/delete),
   logic/session.rs untouched, tests/logic/authorize.rs (reload after direct
   seeding). Red: 89 failures exposed F12 gate mismatches → resolved by schema
   widening; duplicate-entity error → dedup restored. Green: 562/562.
3. S3 gates + docs — DONE: fmt --check clean; clippy zero warnings both
   crates; research docs updated (F12, §11 record); handoff synced.

## 5. Open unknowns
None blocking. Deferred (user-declined or postponed): role-name reservation,
action-EID cosmetic rename, Application entity naming.

## 6. Verification plan
Per-slice crate-local `cargo test -j 1`; full server suite; clippy pedantic;
fmt check. Lab evidence in `document/research/cedr_simulation.md`.

## 7. Risks
- Grant/revoke now takes effect through reload boundary; direct graph writes
  outside logic layer would not auto-reload (none exist in production paths).
- Policy-set size grows with grants (≤39 × roles); bounded by closed vocabulary.

## 8. Constraints
No Cargo.lock edits; no unwrap in production code; one-commit-per-slice when
committing; never discard work.

## 9. Questions for user
- Approve committing as three slices (S1/S2/S3) per workflow §9?
