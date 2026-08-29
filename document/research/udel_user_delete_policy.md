# R: udel — user delete policy: deregister = soft only; delete page is permission-bound

Research code: `udel` — user_delete_policy.

## 1. Requirement (R₀)

R: Self-service deregister is soft-only; account deletion through the
token-less delete endpoint is bound by the permission system plus a
deletion-confirmation request context — never by hard-coded `actor == target`
logic in application code.

Concrete acceptance:

1. Server, deregister path (`delete_user` with `query.token = Some`): always a
   soft delete, the mode value is ignored; the email-confirmation token flow is
   unchanged; the self-service authorization carries request context
   `delete_token_confirmed = true`; response `Empty`.
2. Server, delete path (`query.token = None`): `mode` is required
   (absent -> 400 "missing or unsupported delete mode"); each mode authorizes
   against the **target**:
   - Hard -> `User::Delete::Hard` (role grant only);
   - Transfer -> `User::Delete::Transfer` (role grant only);
   - Soft -> `User::Delete::Soft`; when principal == resource a non-admin is
     denied (the flag is false on this path). Response `UserIdView`.
3. Authorization system: schema declares an optional context attribute
   `delete_token_confirmed: Bool` on `User::Delete::Soft`; handwritten policy
   1b keeps self `Read`/`Update` and gates self `Soft` on the flag; self
   `Transfer` is removed from 1b; grant templates are unaffected. **No new
   action** (schema action count stays 39). The authorizer API accepts the
   flag and serializes it into the Cedar `Context`.
4. Client: deregister page drops the mode picker and submits no mode; a new
   page `/user/{uid}/delete` offers Transfer/Soft/Hard with no email/token,
   a confirm submit, and navigation back to `/user`; linked from the user hub.
5. No behavior change to article/comment/version deletes or undelete flows.

## 2. Research questions

- Q1: can the permission system distinguish "self soft-delete with an
  email-confirmed token" from "self soft-delete via the delete page"?
  (Token presence is invisible to a decision over (principal, action,
  resource) alone.)
- Q2: does Cedar 4.12 support the pieces needed: a request `Context`, an
  optional context attribute in the schema, and a policy condition on it?
- Q3: what currently blocks passing context? Where is the seam in the
  authorizer and server call chain?
- Q4: with self `Transfer` dropped from 1b, which existing flows break?

## 3. Evidence

### Q1 — source
- `code/authorizer/src/authorizer.rs:90-97`: the request is built as
  `Request::new(user, action, resource, Context::empty(), Some(&schema))`.
  Context is currently always empty => the permission system today CANNOT see
  whether a deletion token was carried.
- `code/authorizer/src/authorizer.rs:63-116`: `authorize(principal, action,
  resource)` is the only entry; no context parameter.
- `code/authorizer/cedar/policy.cedar:33-38`: policy 1b permits
  `User::Delete::Soft`/`User::Delete::Transfer` when `principal == resource`.
- `code/server/src/repository/seed.rs:29-40`: ROLE_ADMIN holds every action;
  ROLE_MEMBER holds only read/create grants (no User delete actions).

### Q2 — probe
`test/unit/authorizer/probe_context` (temporary; preserved at
`/tmp/opencode/context_probe.rs`): a standalone Cedar 4.12 harness with the
exact proposed shapes. Result: **6/6 pass**.

| Probe | Context | Decision |
|---|---|---|
| self `Soft`, no flag | `{}` | Deny |
| self `Soft`, flag true | `{"delete_token_confirmed": true}` | Allow |
| self `Soft`, flag false | `{"delete_token_confirmed": false}` | Deny |
| admin -> other `Soft`, no flag | `{}` | Allow |
| member -> other `Soft`, no flag | `{}` | Deny |
| self `Read`, no flag | `{}` | Allow |

Conclusions: optional context attribute `delete_token_confirmed?: Bool`
validates in strict mode; `context has delete_token_confirmed && context.delete_token_confirmed == true`
is valid policy; missing/false flag denies the self soft case; grant templates
without a context condition are unaffected.

### Q3 — source
- `code/authorizer/src/authorizer.rs:94`: `Context::empty()` — the seam.
- `code/server/src/infrastructure/authorizer.rs:50-64`: server wrapper
  `Authorizer::authorize(user_id, action, resource)` delegates to inner.
- `code/server/src/logic/authorize.rs:47-57`: `authorize(state, actor, action,
  resource)` is the logic-layer seam used by `read_delete_user_grant`
  (`code/server/src/logic/user.rs:243` via `authorize_entity`).

### Q4 — source
- `code/server/src/logic/user.rs:281-320`: `handle_delete_user_transfer` and
  `handle_delete_user_soft` are reachable only through a valid email token;
  both authorize via 1b on self.
- `code/server/src/logic/user.rs:184-215`: after the change, the token path
  is always soft, so 1b self `Transfer` has no user and can be removed.
- Destroyed flow: deregister-with-transfer (test `delete_user_transfer_after_email_confirmation`,
  `code/server/src/tests/logic/user.rs:221`) — removed; admin transfer of
  OTHERS is unaffected (grant template).

## 4. Findings

- The authority system CAN own the token-requirement: feed it as request
  context. No new action required (actions stay at 39).
- Without the context flag the self-soft delete page path would succeed for
  any member (1b fires on principal == resource), so wiring context is
  mandatory, not optional.
- Role grants must not reference the flag (admin deletes others without one);
  the probe confirms the template form is unaffected.
- Deregister (token path) keeps 1b `Soft` self; Transfer self is dropped.
- Error-path semantics preserved: missing mode without a token still yields
  the existing 400 message.

## 5. Impact on R

No revision needed. R₀ stands (`R = R₀`).

## 6. Open items

- Context attribute name `delete_token_confirmed` (proposal; renamable).
- Accept that an ADMIN may delete any target, including self, via the delete
  endpoint (administrator holds every grant; the "non-admin" boundary is the
  security line). Non-admin self-delete requires the email-confirmed
  deregister flow only.