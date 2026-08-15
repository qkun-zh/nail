# ADR-0005: Cedar authorization boundary audit

## Status

Accepted (deviation 1A and 2C resolved 2026-08-15; see "Resolution" below)

## Context

The project philosophy (ADR-0001 §2, handoff.md:103) is: **backend does plain
CRUD; authorization — the judgment of "may actor X perform action A on resource
R under the given conditions" — lives entirely in Cedar.** Backend business
code must not re-encode authorization rules.

This ADR records a line-by-line audit of `code/back/src/{logic,interface,
repository}` against that boundary, so the principle has an explicit, reviewable
home and future agents know where the line is drawn. The audit distinguishes
three kinds of code:

- **Authorization** (Cedar's job): who may act.
- **Business invariant** (backend's job): data-integrity rules on a resource's
  own state, independent of who the actor is.
- **Resource CRUD** (backend's job): mechanical persistence.

## Audit result: the write path is compliant

Every mutation entry point funnels through `logic/authorize.rs` and terminates
in `infrastructure/cedar::decide`; no `logic` layer decides authorization
itself:

- `article::{create,update,delete}`, `version::{create,update,delete}`,
  `comment::{create,update,delete}`, `user::{read,update,delete}`,
  `role::*` all call `authorize` / `authorize_or` with a specific Cedar action
  (e.g. `PERMISSION_ARTICLE_DELETE_TRANSFER`) against an explicit `Resource`.
- The single handwritten decision point is `cedar/policy.cedar`; permissions
  flow in through role/permission edges in data, never by editing policies.
- No hardcoded role string (`"admin"`, `"recycler"`, `"member"`) appears in any
  `logic`/`interface` authorization decision. The only references are
  repository data queries (`users_holding_role(ROLE_RECYCLER)`,
  `hold_role(ROLE_MEMBER)`) and seed/bootstrap — reads and mutations of the
  role graph, not authorization checks.

Business invariants are correctly kept in the backend, distinct from
authorization:

- `role.rs:146,202` — `REQUIRED_ROLES` cannot be destructively edited/deleted.
- `article.rs` `reject_duplicate_content_hash`; `version.rs` `NotGreater`,
  `ContentHashTaken`; `transfer.rs` `TargetOwnerMissing` / `NoRecycler`.
- `delete.rs` cascades the graph per resource, not per principal.

None of these are "who may act" decisions; each is about the resource's own
state, so the backend rightfully owns them.

## Deviations to track (not fully practicing the philosophy)

Two read-path spots are not gated through Cedar. Both are currently safe only
because policy rule #2 ("read-open: any authenticated principal") happens to
match, but neither *enforces* that rule via Cedar, so they silently assume it.

1. **PDF content read bypasses Cedar entirely.** `interface/content.rs`
   `read_content` and `logic/download.rs` (`resolve_version_pdf_path`,
   `mint_download_token`, `consume_download_token`) never call `authorize`.
   Access is gated only by the `Principal` extractor (any authenticated user)
   plus existence checks. That equals Cedar rule #2 today, but the endpoint
   does not consult Cedar: if rule #2 were ever tightened (scope- or role-gated
   reads), the content/PDF path would leak without touching a policy. It is the
   same class of gap handoff.md:167 warns about for read-path filtering.

2. **`is_author` uses a permission as a proxy for ownership.** `logic/
   authorize.rs::is_author` answers "may the actor update/transfer this
   resource" (e.g. `PERMISSION_ARTICLE_UPDATE`,
   `PERMISSION_COMMENT_DELETE_TRANSFER`) and exposes it as an `is_author`
   boolean. It is not a raw identity/owner query; it is an authorization
   predicate repurposed as a UI "am I the owner" flag. Today it is coherent
   because the owner-bypass policy grants exactly those actions when
   `resource.owner == principal`, but the coupling is implicit: `is_author` is
   defined by *whatever permission the caller passes*, not by the owner edge.
   A future policy change could silently flip the frontend's ownership affordances.

## Decision

1. The write path and the `authorize`/`cedar::decide` funnel are the reference
   implementation of the philosophy and are accepted as-is.
2. Business invariants stay in the backend; authorization stays in Cedar. This
   ADR fixes that division as the ongoing standard.
3. The two read-path deviations are **not** authority bugs today but are
   latent; they are recorded here so the owner can decide whether to:
   - add an explicit `authorize(... Article::Read / Version::Read ...)` guard in
     the content/download path, and/or
   - redefine `is_author` on the owner edge instead of a passed-in permission.

## Resolution (2026-08-15)

Both deviations were resolved:

- **Deviation 1, option 1A (adopted):** the content/download read path now
  goes through Cedar. `logic/download.rs::resolve_version_pdf_path` — the single
  funnel used by both `mint_download_token` and `consume_download_token` —
  opens with `authorize_or(..., PERMISSION_VERSION_READ, Resource::Version)`
  against the real caller (`actor_id` now threaded through the signature). The
  non-token content read in `interface/content.rs` passes the session principal.
  PDF content access is now enforced by policy rule #2 rather than assumed by it.

- **Deviation 2, option 2C (adopted):** `is_author` was deleted from the
  backend. The response field `is_author` was removed from
  `ArticleView`, `CommentListPage` and `VersionView`; the
  `check_if_is_author` query param, its interface handlers, the
  `logic::authorize::is_author`/`is_allowed` functions and the now-unused
  permission-imports were all removed. Ownership is determined on the client by
  comparing `ArticleView.author_id` against the session user id
  (`session_gate::authenticated_user_id`). Backend read paths no longer repurpose
  an authorization predicate as an ownership flag; the frontend decides ownership
  from the data.

  Note: the "version/comment is_author" affordance previously served by
  `is_author` for non-article resources was not exercised by the frontend and is
  intentionally dropped; if an author affordance is needed there later, compute
  it on the client from the resource's author/user id the same way.

## Consequences

- Reviewers can treat any future `if <role>/<permission>` branch in `logic`
  that decides whether an action is allowed as a regression against ADR-0005.
- The audit gives a concrete, greppable checklist: authorization via
  `authorize`/`authorize_or`/`decide` only; role strings only in repository
  data queries or seed; resource-state rules in the backend.
