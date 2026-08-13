# ADR-0002: `POST /email/read` uses an explicit `intent` query parameter

- Status: accepted (owner-confirmed 2026-08-13, adjudication #26)
- Date: 2026-08-13
- Deciders: project owner

## Context

The legacy `POST /email/read` served three flows — email sign-in, email change
request, and account deregistration request — and selected among them by
inferring intent from which combination of `pow`, `old_email_pow`, and
`new_email_pow` fields were present, and from whether a session token happened
to validate. That inference made the endpoint ambiguous (a valid session plus a
bare `pow` could mean either sign-in or deregistration) and tied the wire
contract to the server's internal session cache state.

## Decision

The endpoint takes an explicit query parameter:

```
POST /email/read?intent=authenticate|change_email|deregister
```

- `intent` is a **query parameter**, not a body field.
- The body stays `EmailReadRequest { pow?, old_email_pow?, new_email_pow? }`;
  the pair `old_email_pow`/`new_email_pow` keeps its both-or-neither invariant.
- A shared `EmailReadIntent` enum lives in `common::request` (serde
  `snake_case`: `authenticate` / `change_email` / `deregister`) so both the
  frontend URL builder and the backend query parser speak the same values.
- `intent` is authoritative: each branch validates only the fields it needs and
  ignores the others. Missing or unknown `intent` is a 400.

## Consequences

- Session-validity is no longer part of the endpoint's dispatch; a session is
  only consulted by the branches that require one (`change_email`,
  `deregister`).
- The sign-in branch (`authenticate`) requires `pow`; the payload is the
  request email. The other two branches land with the user-domain slice.
- Frontend URL building (Phase 4) uses the same enum values, keeping the two
  sides of the contract in one crate.
