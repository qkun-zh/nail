# udel — user delete policy: deregister = soft only; delete page is permission-bound

Code: `udel`.

## 1. Requirement

R: Self-service deregister is soft-only; account deletion through the
token-less delete endpoint is bound by the permission system plus a
deletion-confirmation request context — never by hard-coded `actor == target`
logic.

Acceptance criteria:
- Deregister (`delete_user`, token present) always soft deletes; mode ignored;
  email-token flow unchanged; self-service authorization carries context
  `delete_token_confirmed = true`; response `Empty`.
- Delete endpoint without a token requires `mode` (else 400); per-mode
  authorization against the target (Hard/Transfer grant-only; Soft grant-only
  for others, denied for self without the flag); response `UserIdView`.
- Schema adds an optional context attribute `delete_token_confirmed` on
  `User::Delete::Soft`; policy 1b: self Read/Update kept, self Soft gated on
  the flag, self Transfer removed; grant templates untouched; NO new action
  (count stays 39); authorizer API serializes the flag into the Cedar Context.
- Client: deregister page drops the mode picker; new `/user/{uid}/delete`
  page (Transfer/Soft/Hard, no email/token, confirm, navigate back), router
  route, hub link.
- No article/comment/version delete or undelete behavior change.

## 2. Scope

### In scope
- `code/authorizer/cedar/schema.cedar` (context attr on `User::Delete::Soft`).
- `code/authorizer/cedar/policy.cedar` (1b split + flag condition).
- `code/authorizer/src/authorizer.rs` + principal/resource API: context input.
- `code/server/src/infrastructure/authorizer.rs`: thread context.
- `code/server/src/logic/authorize.rs`: context-aware authorize variant.
- `code/server/src/logic/user.rs`: `delete_user` dispatch + handlers +
  cache clearing; deregister grant check with context.
- `code/server/src/tests/{logic/user.rs,http/user.rs,logic/delete_verify.rs}`
  and authorizer tests; probe promotion.
- Client: `page/user/deregister.rs`, new `page/user/delete.rs`,
  `request/user.rs`, `router.rs`, `page/user/hub.rs`, `page/delete_mode.rs`
  (drop `SOFT_TRANSFER_HARD`).

### Out of scope
- `common/request.rs` `UserDeleteQuery` shape (unchanged).
- Admin-UI probe/trial-and-error decisions; index gating (parked).
- All non-user delete flows.

## 3. Design decisions

- **Context carries the deletion-confirmation fact.** The token itself stays
  an application secret checked against `cache.user_deletion`; the Cedar
  context flag marks the self-service (deregister) path so Cedar's own rule
  (1b + flag) is what protects the email challenge. App asserts the flag only
  on the token-bearing deregister path; delete-page requests assert false.
- **Dispatch on `query.token`, not `query.mode`.** Token present -> deregister
  (always soft, mode ignored, `Empty`). Token absent -> admin delete: mode
  required, per-mode authorize against target, `UserIdView`.
- **1b**: `Read`/`Update` self unchanged; self `Soft` requires the flag; self
  `Transfer` removed (deregister is soft-only; admin transfer rides grants).
- **Grant templates unchanged** (admin-on-others unaffected by context).
- **Cache hygiene**: same as today for deregister; admin modes additionally
  read the target email hash before node removal and clear
  session/user_deletion/email_update/user_creation caches.
- **No new action**: actions stay at 39.

## 4. Slice breakdown

1. **s1 authorizer** — schema + policy + authorizer context input
   (`RequestContext { delete_token_confirmed }`, serialized to Cedar Context),
   probe promoted into `tests/`. Exit: `cargo test -p authorizer`.
2. **s2 server core** — infrastructure + logic authorize variants; `delete_user`
   rewrite (deregister/soft; admin hard/transfer/soft with per-target
   authorize + caches). Exit: `cargo test -p server`
   (updated tests must pass).
3. **s3 server tests** — rewrite `tests/logic/user.rs`, `http/user.rs`,
   `delete_verify.rs` user cases; add admin-mode and self-denial cases.
   Exit: `cargo test -p server -p authorizer`.
4. **s4 client** — deregister no-mode; new `/user/:uid/delete` page; route;
   hub link; drop `SOFT_TRANSFER_HARD`. Exit: `cargo test -p client`,
   `trunk build`.

Each slice: fmt + clippy clean, one commit, push, CI green.

## 5. Open unknowns

- Context attribute naming (`delete_token_confirmed`) — cosmetic, renamable.
- None blocking; cedar context semantics proven by probe (§4 of research doc).

## 6. Verification plan

- Per slice: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`,
  targeted tests, `trunk build` (s4), push + CI.
- Final: full `cargo test -j 1` for server/common/emailer/client + authorizer,
  CI green, then delete this doc and the research report.

## 7. Risks

- Forgetting context wiring on the deregister path would lock everyone out of
  self-service (flag false). Mitigated by tests.
- Admin retains full delete power including self (by design — the security
  boundary is the non-admin line).
- Semantic change for stale clients sending a mode on deregister: ignored
  (soft). Server-side safe direction.

## 8. Constraints

- English only; one commit per slice; clean tree; no unwrap/expect/panics;
  no sed/awk edits; never touch Cargo.lock; never discard work; §8 adoption
  required before any code.
- No changes outside the listed scope; admin-UI items stay parked.

## 9. Questions

- Q1 (adoption gate): proceed with the four slices above?
- Q2: accept that an admin may delete any account via the delete endpoint,
  including their own (grant covers all modes, no flag required)?

## Change log

- 2026-08-29: rewritten from the discarded new-action design to the
  request-context design after user direction (permission system must own the
  distinction; no new action).