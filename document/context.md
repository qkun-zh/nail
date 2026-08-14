# nail_new

A forum for publishing PDF documents as versioned articles, with email-based
sign-in, threaded comments, full-text search, and role-based access control.
This file is the glossary only; it carries the ubiquitous language shared by
the code, the tests, the PRDs, and every design document.

## Language

### Content

**Article**:
A published document made of a title, a summary, tags, and an ordered list of
versions, owned by exactly one user.
_Avoid_: post, thread, entry, document

**Version**:
One release of an article: a PDF together with its semver number and a short
note. Every version belongs to exactly one article.
_Avoid_: revision, upload, file

**Comment**:
A short text attached to a version (top-level) or to another comment (reply),
owned by its author. A reply is a comment whose parent is a comment.
_Avoid_: post, message

**Tag**:
A `#name` marker attached to articles for classification and to roles for
scoping.
_Avoid_: label, category, hashtag

**Search hit**:
A snippet of one article field matching a query, together with that field's
display label and the article it belongs to.

**Download token**:
A single-use, short-lived token (TTL 60 s) bound to one version's PDF and one
user; redeeming it serves that PDF's bytes.
_Avoid_: download link, one-time URL

### Identity and sign-in

**User**:
A registered person, identified by the hash of their email address; every user
holds at least the member role.
_Avoid_: account, profile

**User zero**:
The initial user created at first boot from the configured email address,
holding all required roles.

**Session token**:
The credential that identifies an authenticated user for a bounded time.
_Avoid_: auth token, bearer token, cookie

**Email verification token**:
A one-time token emailed to an address; redeeming it proves control of that
address.
_Avoid_: one-time code, OTP

**Challenge**:
A server-issued, single-use puzzle whose solution is a proof of work.
_Avoid_: nonce, puzzle

**Proof of work**:
A MinRoot VDF evaluation over a challenge and a request-specific payload,
proving real computation.
_Avoid_: CAPTCHA, rate-limit token

**Intent**:
The query parameter that selects which flow `POST /email/read` runs:
`authenticate`, `change_email`, or `deregister`.
_Avoid_: mode, purpose, branch

### Authorization

**Role**:
A named bundle of permissions and tag scopes that users hold. The required
roles are admin, recycler, and member.
_Avoid_: group, access level, rank

**Permission**:
A single action a role may perform (create/read/update/delete on each resource,
plus role management).
_Avoid_: right, privilege, capability

**Scope**:
The set of tags a role applies to; a holder may act only on resources carrying
one of those tags. A role without scopes is global.
_Avoid_: domain, range, boundary

**Owner**:
The user to whom a resource's ownership edge points.
_Avoid_: author (authorship is fixed at creation; ownership can be transferred)

**Member**:
The default role every user holds.

**Recycler**:
A user holding the recycler role; transferred resources are re-pointed to the
least-loaded recycler.
_Avoid_: trash account, graveyard keeper

**Admin**:
A user holding the admin role, allowed every action.

### Deletion semantics

**Transfer**:
The delete mode that re-points a resource's ownership edge to the least-loaded
recycler instead of removing it.
_Avoid_: soft delete, recycle

**Hard delete**:
The delete mode that removes the resource and everything belonging to it.
_Avoid_: purge, cascade delete

### Communication

**Envelope**:
The wrapper of every backend response, holding a code, a human-readable
message, and optional data.
_Avoid_: payload, body, wrapper
