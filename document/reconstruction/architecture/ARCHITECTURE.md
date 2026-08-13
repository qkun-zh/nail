# Architecture

| Setting | Value |
| --- | --- |
| Mode | `redesign` |
| Level | `complex` |
| Fidelity | `describe` |
| TDD | `on` (build test-first) |
| Generated with | `reconstruct@2.17.0` |

## Detected stack

Rust · three crates: `common` (shared), `back` (axum server), `front` (Leptos CSR / wasm).
The engine detected no framework label, but the Cargo manifests (see
`data/config/`) pin the actual stack: **axum** backend embedding **agdb** (graph
db), **seekstorm** (full-text search), **moka** (cache), **cedar-policy**
(authorization); **Leptos** frontend in CSR mode; **pingap** reverse proxy
serving static assets and `/api` (rate limiting, body limits, access logs live
in the proxy layer); **trunk** builds the frontend; PoW uses **pso-vdf**
(MinRoot), hashing uses **ascon-xof128**.

## Top-level layout (observed)

- `code/back` — axum server: `api/` (handlers + router), `logic/` (business rules),
  `repo/` (agdb + moka + seekstorm access), `other/` (app_state, conf, email,
  log, pdf, server), `authorization/` (Cedar entity store + gate + schema/policy)
- `code/common` — shared: hash, name, pow, request, response, search, tag, text, time
- `code/front` — Leptos CSR: `page/` (UI), `req/` (HTTP), `router/`, conf, limits, pow
- `conf/` — toml configs (back server/email/smtp, front compile-time, proxy pingap)
- `data/` — runtime: agdb graph, seekstorm index, PDF store

## External services & integrations

- **SMTP (qq.com)**: `smtp.qq.com:587`, username `qkun-zh@qq.com`; command-level
  timeout 10s, wall-clock timeout 30s; per-recipient rate limit (moka-backed);
  failures are hard errors (`SendEmailError::RateLimited` → 400, `Smtp` → 500).
  Email verification tokens are sent as mail with a single-use redemption flow
  (auth / email change / deregister branches). Allowed domains:
  qq.com, foxmail.com, 163.com, 126.com, gmail.com, outlook.com.
- **Local PDF storage**: `pdf_storage_path`; versions store PDF bytes on disk,
  content-addressed by ascon-xof128 hash; serving via `content/read` (inline /
  minted single-use download token, TTL 60s).
- **agdb graph database**: `db_path`, namespace/database from conf (namespace
  fields are never read — dead config, see notes); single writer with locking;
  index-lookup-based unique enforcement.
- **seekstorm search index**: `search_index_path`; derived data (rebuildable);
  full index rebuild at every startup (O(N), deliberate per logs).

## Cross-cutting policies

- **PoW**: MinRoot VDF, `pow_difficulty_iterations = 8192`; challenge TTL 300s,
  single-use, server-issued via `/challenge/read`; required on sensitive user
  operations (register/login/rename/logout/deregister/email change) — not on
  management endpoints (session + permission gate instead).
- **Token TTLs**: email auth token 8000s; session 8000s; download token 60s
  single-use; email-update tokens one per user (replaced on re-request).
- **Validation**: title ASCII 1..=200; summary 1..=2000 (newlines ok); version
  note 1..=1024; tags 1..=8 hashtags `#xxx` 2..=32 ascii alnum/-/_ after `#`;
  comment content 1..=1024; role name 1..=64 ascii alnum/-/_; PDF ≤32MiB,
  must start `%PDF-` (1.x/2.x) and end `%%EOF`; multipart text fields each ≤1MiB
  (`max_text_field_bytes`); body limit = pdf + 5×text + 64KiB.
- **Rate limiting**: at the pingap proxy (`conf/proxy/plugins.toml`), not the
  backend.
- **Authorization**: Cedar `schema.cedar` (16 actions) + `policy.cedar`
  (owner / public-visibility / role+scope / admin-console / admin override);
  `entity_store` assembles per-request entities; roles: admin (full),
  member (create article/comment, auto-granted at registration), recycler
  (transfer recipient), custom (Role::Manage).
- **Session**: moka cache, keyed by hash(session-token), reverse index by user;
  `session-token` header; TTL 8000s.

## Proposed architecture (redesign)

The target project (`nail_new`, see its README) keeps the same features and
behavior but restructures both sides into strict layered architectures with
unidirectional dependencies. Mapping from the observed layers:

```text
Observed (nail)                    Target (nail_new)
─────────────────────────────      ─────────────────────────────
code/back/src/api/*                back/src/interface/*   (HTTP surface,
  (handlers + router + envelope)     envelope {code,data,message},
                                     session-token, PoW gate)
code/back/src/logic/*              back/src/logic/*      (business rules,
  (business rules)                   error mapping at boundary)
code/back/src/repo/*               back/src/repository/* (agdb + moka + seekstorm
  (agdb + moka + seekstorm)          data access, cache design)
code/back/src/other/*              back/src/infrastructure/*
  (app_state, conf, email, log,      (axum wiring, conf loading, SMTP client,
   pdf, server)                      PDF store, logging, server bootstrap)
code/back/src/authorization/*      logic/interface-facing authorization
  (Cedar entity store + gate)        domain + infrastructure Cedar engine
code/common/src/*                  common/src/*          (shared DTOs: envelope,
  (hash, pow, request, response,     pow, validation, time, search shapes;
   search, tag, text, time)          renamed from xxx/yyy/zzz placeholders)
code/front/src/page/*              front/src/page/*      (UI + local state)
code/front/src/req/*               front/src/request/*   (HTTP calls, envelope
  (HTTP + session token)             unwrap, token handling)
code/front/src/router.rs           front/src/router/*    (URL → page only)
code/front/src/conf, limits, pow   front/src/infrastructure/*
  (compile-time config, limits,      (wasm primitives, runtime config fetch,
   pow computation)                  storage, PoW)
```

Rationale per module:

- **interface** owns every HTTP concern: routing, the `{code,data,message}`
  envelope, `session-token` extraction, PoW verification placement, and the
  final error mapping. It depends on `logic` and `infrastructure` only.
- **logic** holds pure, testable business rules (prefer near-pure functions per
  the README) and converts errors only at its boundary. It depends on
  `repository` and `infrastructure`.
- **repository** is the single owner of agdb/moka/seekstorm access; cache
  key layout and graph edge design stay faithful to the observed design
  (they are the strong references).
- **infrastructure** holds the heavy external wiring (axum bootstrap, toml conf
  loading, SMTP client with timeouts, PDF store, tracing logging, Cedar engine).
- **frontend**: router maps URL → page; page renders and holds local state,
  reaching the backend only through `request`; `request` owns every HTTP call,
  session-token handling and envelope unwrapping; `infrastructure` holds
  browser/wasm primitives (compile-time config, runtime config fetch from
  `/config/read`, storage, PoW), reachable from both page and request.
- **common** is dependency-free (depends on nothing internal) and holds shared
  data structures first, per the design-order rule (data structures before
  business logic).

Behavior is preserved; the redesign targets testability (seams at
interface/logic/repository/infrastructure), clarity, and elimination of the
dead paths enumerated in the feature PRD notes.
