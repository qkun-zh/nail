# Authorization model (nail)

Durable record of how access is enforced. The refactor history and slice
tracking live in `document/authz-refactor.md`; this file is the stable model
description that `README.md` §5 points at.

## Layered model

Access is enforced in three layers, in order on every request:

1. **Identity / session** — email-challenge + PoW proof yields a session token
   (TTL cache, `logic/session.rs`). Every request must present a valid
   `session-token` header (`interface/principal.rs`); the principal is resolved
   from the session before anything else. This is the "is someone there, and
   who" layer.
2. **Authorization** — `logic/authorize.rs` is the single enforcement entry
   (O2): it assembles the actor's graph (user, held roles, role permission
   edges) into Cedar entities (`repository/authorization.rs`) and calls
   `infrastructure/cedar::decide` against `policy.cedar` + `schema.cedar`. All
   writes and management operations are judged here. The action vocabulary is
   a single source: `schema.cedar` declares the 27 actions, and seeding derives
   the permission nodes from it (`schema_actions()`), so policy, schema, and
   data cannot drift (A2/A5).
3. **One-time token binding** — PDF download is the one path that cannot ride a
   long-lived principal session: `logic/download.rs` mints a short-lived,
   single-use token (`mint_download_token`) bound to the requesting account and
   version; `consume_download_token` checks the binding, consumes the token, and
   re-authorizes through Cedar (`Version::Read`) before serving bytes.

The three layers compose: a request is a session-authenticated actor whose
write/management actions pass Cedar, and whose rare out-of-band file transfer
uses a bound, consumable token instead of the session itself.

## Read gating today

Reads are Cedar-gated (B1): single-resource reads authorize
`Article::Read` / `Version::Read` / `Comment::Read` against the resource, and
collection reads authorize once against the coarse `Virtual::"read"` desk (the
former read-open policy 2 was removed in B1; policy numbering is stable). Any
authenticated principal may no longer read: a non-member is denied (403) and a
member reads via the seeded D5 read grants; the PDF download path already
authorizes `Version::Read` at `logic/download.rs:19`.

The B1 thread: `actor_id` runs through the read entry points (`logic/article.rs`
`read_article`, `logic/search.rs` `search_articles`, `logic/version.rs`
`read_version`/`read_versions`, `logic/comment.rs` `read_comments`/`read_comment`/
`read_comment_children`) and their interface handlers.

## Decision records

- **D1 — scope axis removed**: roles are pure sets of users. The scope mechanism
  (`role_apply_tag`, `scopes`, `global_role`, `required_scopes`, the `Tag` in
  schema) is deleted; tags are content metadata only. This removes the
  escalation hazard where a content tag silently granted a permission and the
  conflation of "role membership" with "content scoping" (A1).
- **D3 — admin power is data, not a rule**: policy 6 (`permit ... principal in
  Role::"admin"` override) is deleted. Admin holds every schema action as an
  explicit data grant, seeded from the schema itself, so a new action is granted
  to admin at next startup by the same seed loop with no policy edit. No policy
  grants power to a role (A5).
- **D7 — admin revocation is blocked in policy**: revocation is its own action
  (`Role::Revoke`), judged against the **target** role as resource, with
  `forbid(principal, action == Role::Revoke, resource == Role::"admin")`.
  `Role::Manage` still covers create/read/add-permission/delete against
  `Virtual::"admin-console"`. The Rust required-role guard still protects
  recycler/member; its admin coverage was removed so the forbid is the sole
  admin protection and can never be shadowed by the guard (A4).
