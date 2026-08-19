# Exec — Task VII-routes: singularize 4 public comment routes (A1–A4)

**Task**: VII-routes (REFACTOR_PLAN.md §3, approved A1–A4). **Owner**: uD7wXk.
**Scope root**: `code/back/src/interface/router.rs`, `test/unit/back/http/comment.rs`,
`test/unit/back/infrastructure/cedar.rs`, `code/front/src/request/comment.rs`.

## 1. Requirement

USER-APPROVED public API change: replace plural resource segments with singular
in 4 comment routes, updated in lockstep across backend constant, backend
tests, and frontend path builder so the tree stays green:

| # | old | new |
|---|---|---|
| A1 | `/version/{id}/comments/create` | `/version/{id}/comment/create` |
| A2 | `/version/{id}/comments/read` | `/version/{id}/comment/read` |
| A3 | `/comments/{id}/replies/create` | `/comment/{id}/reply/create` |
| A4 | `/comment/{id}/replies/read` | `/comment/{id}/reply/read` |

Acceptance: back http + infrastructure tests green; full back split green
(576: configuration 11, infrastructure 45, logic 273, repository 108, http
139); fmt clean; clippy 0 warnings; frontend compiles (check only). No
function renames, no query-param changes, no other edits.

## 2. Scope

In: the 4 files above. Out: docs (`the_shit.md`, `REFACTOR_PLAN.md`),
`test/unit/front/request/url/tests.rs` (generic url-builder test using
`"comments"` as sample data — not a route path), any other agent's files.

## 3. Design decisions

- Constant renames (paths no longer match names, so names mislead):
  `ROUTE_VERSION_ID_COMMENTS_CREATE` → `ROUTE_VERSION_ID_COMMENT_CREATE`;
  `ROUTE_VERSION_ID_COMMENTS_READ` → `ROUTE_VERSION_ID_COMMENT_READ`;
  `ROUTE_COMMENTS_ID_REPLIES_CREATE` → `ROUTE_COMMENT_ID_REPLY_CREATE`;
  `ROUTE_COMMENT_ID_REPLIES_READ` → `ROUTE_COMMENT_ID_REPLY_READ`.
  No name collisions (existing: `ROUTE_COMMENT_ID_READ/UPDATE/DELETE/
  UNDELETE_SOFT`). Grep confirms the constants are referenced only in
  `router.rs` (defs + route wiring) and `cedar.rs` (import + assertion), so
  the rename is fully contained.
- `test/unit/back/infrastructure/cedar.rs:125-145`
  (`generated_route_constants_match_their_literal_paths`) asserts the A1
  constant against its literal — updated in the same slice as router.rs
  (implementation-consistency test, not a behavior test).
- Frontend `code/front/src/request/comment.rs` builds paths from segment
  arrays (`url::build_path_with_query`) — swap `"comments"`→`"comment"`,
  `"replies"`→`"reply"` at the 4 sites (lines 15, 34, 42, 55). Function names
  and query params untouched.

## 4. Slice breakdown

### Slice 1 — coordinated 4-file change (one commit)

- **Goal**: all 4 routes singularized across backend, tests, frontend.
- **Files**: `code/back/src/interface/router.rs`,
  `test/unit/back/http/comment.rs`, `test/unit/back/infrastructure/cedar.rs`,
  `code/front/src/request/comment.rs`.
- **Red**: update the 30 hardcoded path literals in
  `test/unit/back/http/comment.rs` to the new paths; `cargo +nightly test
  http_comment` must fail (new paths not yet routed → 404).
- **Green**: router.rs paths + constant renames; cedar.rs import + literal;
  frontend segment swaps. `cargo +nightly test http_comment` +
  `infrastructure_cedar` pass.
- **Exit test**: `cargo +nightly test http_comment && cargo +nightly test
  infrastructure_cedar` green; then full back split (configuration,
  infrastructure, logic, repository, http) + `cargo fmt --check` +
  `cargo clippy -D warnings`; frontend `cargo +nightly check`.

## 5. Open unknowns

None. Pure string change; no behavior, no dependencies, no evidence loop
needed (source = repo grep below; behavior visible in source).

Grep inventory (repo scope, excluding target/dist/data/log/.git/Cargo.lock):

- `code/back/src/interface/router.rs:34,35,36,38` — 4 constants (code).
- `test/unit/back/http/comment.rs:86,104,113,129,137,155,175,189,213,226,
  240,252,260,275,284,303,337,362,394,433,467,492,507,526,546,559,573,609,
  634,660` — 30 hardcoded literals (code).
- `test/unit/back/infrastructure/cedar.rs:129,138` — constant import +
  literal assertion (code).
- `code/front/src/request/comment.rs:15,34,42,55` — 4 segment arrays (code).
- `document/REFACTOR_PLAN.md:20-23`, `the_shit.md:141` — docs, NOT changed.
- `test/unit/front/request/url/tests.rs:35` — generic builder test, sample
  segment `"comments"` not a route; NOT changed.

## 6. Verification plan

| Dimension | How verified |
| --- | --- |
| Correctness | http_comment red (404) then green; cedar literal test green |
| Behavior change | delta = exactly the 4 route strings (grep shows no other code hit) |
| Time complexity | N/A — constant strings, no algorithm touched |
| Space complexity | N/A — no allocation change |
| Performance | N/A — no runtime path change beyond string equality |

## 7. Risks

- Concurrent agent touches `code/front/src/request/comment.rs` or back test
  files → re-read files before editing; STOP + report if conflict surfaces.
- Full back test run OOMs in one binary → run per-module (REFACTOR_PLAN §2).
- Rollback: revert the single slice commit (orcherstrator side).

## 8. Constraints

- Only the 4 listed files; no docs edits; no frontend function renames; no
  query-param changes; no `unwrap`/`expect`; no comments; English only.
- Machine load check before every build; never `--release`; never touch
  `target/dist/data/log/Cargo.lock`; no `git add -A`.

## 9. Questions

None.

## Change log

- 2026-08-19: Adoption approved by orchestrator. Slice 1 executed. Red phase:
  22 http_comment tests failed with 404 on the new paths as designed. Green
  phase: a red-phase gap surfaced — the 5 A3 literals kept the `/comments/`
  prefix after the `replies→reply` replaceAll (A3 also singularizes the
  prefix); fixed before the gate. Gate: http_comment 23/23,
  infrastructure_cedar 22/22, full back split 576/576 (configuration 11,
  infrastructure 45, logic 273, repository 108, http 139), fmt clean,
  clippy 0 warnings, frontend `cargo +nightly check` clean. Committed as one
  slice commit.