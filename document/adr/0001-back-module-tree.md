# ADR-0001: Backend module tree

- Status: accepted
- Date: 2026-08-13
- Deciders: migrating agent (owner mandate via README §4.1)

## Context

`nail_new` fixes the four backend layers (`interface`, `logic`, `repository`,
`infrastructure`) and their dependency direction, but leaves the module trees
beneath each layer unspecified. README §4.1 forbids copying the legacy `nail`
backend division (`api.rs`-style route surface, `repo/`, `authorization/`) and
demands every boundary be justified by the new layer's responsibilities and
callers.

## Decision

Each layer is split into leaf modules sized so that one responsibility has one
owner, and so that each layer's modules mirror the *caller* they serve rather
than any legacy arrangement:

```
interface/        one module per HTTP concern, split by route family
  router.rs         the route table (paths -> handlers)
  envelope.rs       ApiError -> {code, data, message} mapping
  principal.rs      session-token extractor -> authenticated user id
  pow.rs            proof-of-work gate placement
  challenge.rs      GET /challenge/read
  email.rs          POST /email/read (intent dispatch)
  registration.rs   POST /user/create
  session.rs        GET /session/read, POST /session/delete
  (user/article/version/comment/role/content land with their slices)

logic/            one module per business capability, near-pure
  error.rs          LogicError (the typed error callers distinguish)
  challenge.rs      challenge issuance
  pow.rs            proof-of-work verification (challenge consume + solution)
  email.rs          email request flow (intent dispatch, email validation)
  authenticate.rs   email-token exchange and session-token validation
  session.rs        session read fields and logout
  (user/article/version/comment/role/search/download/authorization later)

repository/       one module per data owner
  graph.rs          agdb handle + query primitives (the access core)
  schema.rs         node shapes, edge names, key constants
  seed.rs           first-boot seeding (indexes, permissions, roles, user zero)
  user.rs           user node queries
  role.rs           role/permission edge mutations
  cache.rs          moka token caches (one generic single-use token cache)
  (article/comment/tag/search/transfer/delete/authorization later)

infrastructure/   one module per external dependency or bootstrap concern
  config.rs         toml loading + validation (config/{server,smtp,email}.rs)
  email.rs          EmailSender seam + rate limiting (email/smtp.rs transport)
  logging.rs        tracing setup, per-minute files, retention prune
  server.rs         axum bootstrap + graceful shutdown
  state.rs          AppState composition
  (pdf.rs / cedar.rs land with their slices)
```

## Rationale

- **Caller-mirrored seams.** An `interface` handler translates one HTTP route
  into one `logic` call; grouping handlers by route family means each handler's
  module is where its routes change. The legacy split (`article.rs` vs
  `article_view.rs`, `meta.rs`) separated views from mutations and renamed
  config to "meta"; neither distinction serves any caller here, so both are
  dropped.
- **Depth over mirroring.** The legacy backend spread one capability across many
  shallow files: six near-identical token-cache modules under `repo/token/`,
  plus `db.rs` + `schema.rs` + `types.rs` for graph access. `cache.rs`
  collapses the six caches into one generic single-use token cache behind a
  three-method interface (`insert`, `consume`, `delete_by_reverse`); `graph.rs`
  + `schema.rs` + `seed.rs` split by responsibility (access / shape / first
  boot) rather than by legacy file. The gain is locality: the cache pattern,
  the graph schema, and the seed order each live in exactly one place.
- **Seeding is not schema.** `seed.rs` (first-boot writes) is separated from
  `schema.rs` (data shapes), because they have different callers: everything
  reads `schema`, only boot reads `seed`.
- **Email sending is a real seam.** Production (SMTP) and tests (recording
  sink) are two adapters over the `EmailSender` trait, so rate limiting is
  factored into a transport-agnostic wrapper instead of being glued to SMTP.

## Consequences

- Adding a domain slice touches one module in each layer (e.g. the auth slice
  touches `interface/{challenge,email,registration,session}.rs`,
  `logic/{challenge,pow,email,authenticate,session}.rs`,
  `repository/{graph,schema,seed,user,role,cache}.rs`,
  `infrastructure/{config,email,logging,server,state}.rs`), which matches the
  slice-by-slice migration order in `document/handoff.md`.
- Module names that are not yet implemented appear only when their slice
  arrives; no empty placeholder modules are committed.
- The generic token cache introduces one small trait (`CacheEntry`) for reverse
  indexing; every token family is a `TokenCache<E>` instance, so a regression
  in one family is caught by the shared cache tests.
