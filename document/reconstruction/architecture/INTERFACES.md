# Interface surface

| Setting | Value |
| --- | --- |
| Mode | `redesign` |
| Level | `complex` |
| Fidelity | `describe` |
| TDD | `on` (build test-first) |
| Generated with | `reconstruct@2.17.0` |

> 🧠 **For the AI agent:** Enumerate **every** interface this project exposes — HTTP routes, REST/JSON endpoints, tRPC/gRPC procedures, GraphQL operations, CLI commands, scheduled jobs, queues, and webhooks. The deterministic engine resolves routes for the supported frameworks (Next.js, Express, Fastify, Hono, Flask, FastAPI, NestJS, Django, Rails, Go); for everything else, **read the candidate files below** and follow `references/analysis-playbook.md` (§Interface surface) plus the matching guide in `references/stack-guides/`. Fill the target table with one row per operation.


## Resolved routes (deterministic — verify against source)

_None resolved deterministically — read the candidate files below to map the surface._

## Route candidates (verify — may include false positives)

- `code/back/src/api.rs`
- `code/back/src/api/article.rs`
- `code/back/src/api/article_view.rs`
- `code/back/src/api/authenticate.rs`
- `code/back/src/api/comment.rs`
- `code/back/src/api/meta.rs`
- `code/back/src/api/role.rs`
- `code/back/src/api/user.rs`
- `code/back/src/api/version.rs`
- `test/unit/back/http/article.rs`
- `test/unit/back/http/article_delete.rs`
- `test/unit/back/http/article_pdf.rs`
- `test/unit/back/http/article_search.rs`
- `test/unit/back/http/article_update.rs`
- `test/unit/back/http/authenticate.rs`
- `test/unit/back/http/author.rs`
- `test/unit/back/http/comment.rs`
- `test/unit/back/http/comment_delete.rs`
- `test/unit/back/http/download.rs`
- `test/unit/back/http/email.rs`
- `test/unit/back/http/user.rs`
- `test/unit/back/http/version.rs`
- `test/unit/back/logic/article.rs`
- `test/unit/back/logic/article_search.rs`
- `test/unit/back/logic/authenticate.rs`
- `test/unit/back/logic/author.rs`
- `test/unit/back/logic/comment.rs`
- `test/unit/back/logic/download.rs`
- `test/unit/back/logic/email.rs`
- `test/unit/back/logic/user.rs`
- `test/unit/back/logic/version.rs`
- `test/unit/back/repository/article.rs`
- `test/unit/back/repository/authorization.rs`
- `test/unit/back/repository/comment.rs`
- `test/unit/back/repository/search.rs`
- `test/unit/back/repository/tag.rs`
- `test/unit/back/repository/transfer.rs`

## API surface candidates (tRPC / GraphQL / gRPC / OpenAPI)

_No RPC/GraphQL/OpenAPI candidates detected._

## Realtime / WebSocket candidates (verify)

_No realtime/WebSocket signals detected._

## Auth / middleware candidates (verify)

_No auth/middleware signals detected — still record the auth rule per operation below._

## Interface table (fill this in)

| Method / Trigger | Path / Operation | Kind | Handler file | Auth | Notes |
| --- | --- | --- | --- | --- | --- |

> 🧠 **For the AI agent:** Keep these columns; add a row per operation. Note auth/permission requirements, input/output shapes (link to `DATA-MODEL.md`), and side effects.

