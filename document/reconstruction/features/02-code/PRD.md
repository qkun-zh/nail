# Code

> Unit `02-code` · kind: feature

## Summary

Groups 120 file(s).

## Context & goal

> 🧠 **For the AI agent:** State this unit's user-facing goal in 1–2 sentences (the outcome a user gets), and name the other units it depends on and that depend on it. Derive it from the source material below.


## User stories

> 🧠 **For the AI agent:** Enumerate **every** actor and what they need, one line each — `As a <role>, I can <action> so that <value>.` Be **exhaustive**: cover every role and every distinct behaviour, not just the happy path. This list is the backbone of the PRD; nothing below should exist without a story above it.


## Functional requirements

> 🧠 **For the AI agent:** Turn the stories into a **numbered** checklist of precise, testable behaviours, derived from the source material below. Cover happy paths, every edge case, every validation rule, and every error state. Leave nothing as "etc." or "and so on" — if you write a placeholder, you are not done. Tag each requirement `[confirmed]` (read directly in the source), `[inferred]` (pattern-derived, no false certainty), or `[gap]` (needs a human) so the `--verify` pass can adjudicate its confidence faster.


## Interfaces & data

> 🧠 **For the AI agent:** List **every** operation this unit exposes with its input/output shape (link `../../architecture/INTERFACES.md`), and **every** entity it reads or writes (link `../../architecture/DATA-MODEL.md`). Spell out the **write contract** for each mutation: which entities are written, whether the write is transactional, and — for every required (NOT NULL, no-default) column and foreign key — where the value comes from. A public/anonymous operation cannot satisfy an owner foreign key: it must write to an anonymous-capable entity instead. Every enum/domain value it accepts must be one of the members enumerated in `DATA-MODEL.md`.


## Acceptance criteria

> 🧠 **For the AI agent:** Write **Given / When / Then** scenarios that gate "done" — at least one per functional requirement, **including** the failure paths. Example: `Given an unauthenticated visitor, When they POST a todo, Then the API responds 401 and writes nothing.` These scenarios are the spec the rebuild is verified against.


## Edge cases & failure modes

> 🧠 **For the AI agent:** Enumerate what can go wrong and the expected behaviour for each: invalid / empty / oversized input, auth & permission failures, concurrency / race conditions, missing or slow dependencies, partial failures, and idempotency / retries. Each row here should map to an error-path requirement above.


## Test plan (write these first)

> 🧠 **For the AI agent:** Before writing any implementation, turn the functional requirements and acceptance criteria above into failing tests (red): one per behaviour — happy paths, edge cases, validation, and error states. Implement only enough to make them pass (green), then refactor. List the test cases here as a checklist.


## Source material

Files that implement this unit (rewrite them from the requirements above):

- `code/back/src/api.rs`
- `code/back/src/api/article.rs`
- `code/back/src/api/article_view.rs`
- `code/back/src/api/authenticate.rs`
- `code/back/src/api/comment.rs`
- `code/back/src/api/meta.rs`
- `code/back/src/api/role.rs`
- `code/back/src/api/user.rs`
- `code/back/src/api/version.rs`
- `code/back/src/authorization.rs`
- `code/back/src/authorization/entity_store.rs`
- `code/back/src/authorization/gate.rs`
- `code/back/src/authorization/schema.cedar`
- `code/back/src/logic.rs`
- `code/back/src/logic/article.rs`
- `code/back/src/logic/article_search.rs`
- `code/back/src/logic/authenticate.rs`
- `code/back/src/logic/author.rs`
- `code/back/src/logic/comment.rs`
- `code/back/src/logic/download.rs`
- `code/back/src/logic/email.rs`
- `code/back/src/logic/error.rs`
- `code/back/src/logic/role.rs`
- `code/back/src/logic/user.rs`
- `code/back/src/logic/version.rs`
- `code/back/src/main.rs`
- `code/back/src/other.rs`
- `code/back/src/other/app_state.rs`
- `code/back/src/other/conf.rs`
- `code/back/src/other/email.rs`
- `code/back/src/other/email/email_core.rs`
- `code/back/src/other/log.rs`
- `code/back/src/other/pdf.rs`
- `code/back/src/other/server.rs`
- `code/back/src/repo.rs`
- `code/back/src/repo/article.rs`
- `code/back/src/repo/article/edge.rs`
- `code/back/src/repo/article/version.rs`
- `code/back/src/repo/article/view.rs`
- `code/back/src/repo/authorization.rs`
- `code/back/src/repo/comment.rs`
- `code/back/src/repo/db.rs`
- `code/back/src/repo/hard_delete.rs`
- `code/back/src/repo/schema.rs`
- `code/back/src/repo/search.rs`
- `code/back/src/repo/tag.rs`
- `code/back/src/repo/token.rs`
- `code/back/src/repo/token/authenticate.rs`
- `code/back/src/repo/token/challenge.rs`
- `code/back/src/repo/token/deregister.rs`
- `code/back/src/repo/token/download.rs`
- `code/back/src/repo/token/email_update.rs`
- `code/back/src/repo/token/session.rs`
- `code/back/src/repo/transfer.rs`
- `code/back/src/repo/types.rs`
- `code/back/src/repo/user.rs`
- `code/back/src/repo/util.rs`
- `code/common/src/bin/prove.rs`
- `code/common/src/hash.rs`
- `code/common/src/lib.rs`
- `code/common/src/name.rs`
- `code/common/src/pow.rs`
- `code/common/src/request.rs`
- `code/common/src/response.rs`
- `code/common/src/search.rs`
- `code/common/src/tag.rs`
- `code/common/src/text.rs`
- `code/common/src/time.rs`
- `code/front/src/conf.rs`
- `code/front/src/limits.rs`
- `code/front/src/main.rs`
- `code/front/src/page.rs`
- `code/front/src/page/auth_gate.rs`
- `code/front/src/page/index.rs`
- `code/front/src/page/not_found.rs`
- `code/front/src/page/notify.rs`
- `code/front/src/page/pagination.rs`
- `code/front/src/page/private.rs`
- `code/front/src/page/private/authenticate.rs`
- `code/front/src/page/private/deregister.rs`
- `code/front/src/page/private/email.rs`
- `code/front/src/page/private/email/index.rs`
- `code/front/src/page/private/email/update.rs`
- `code/front/src/page/private/index.rs`
- `code/front/src/page/private/layout.rs`
- `code/front/src/page/private/logout.rs`
- `code/front/src/page/private/name.rs`
- `code/front/src/page/private/name/update.rs`
- `code/front/src/page/public.rs`
- `code/front/src/page/public/article.rs`
- `code/front/src/page/public/article/comment.rs`
- `code/front/src/page/public/article/comment/fetch.rs`
- `code/front/src/page/public/article/comment/pagination.rs`
- `code/front/src/page/public/article/comment/render.rs`
- `code/front/src/page/public/article/comment/style.rs`
- `code/front/src/page/public/article/comment/url.rs`
- `code/front/src/page/public/article/create.rs`
- `code/front/src/page/public/article/delete.rs`
- `code/front/src/page/public/article/detail.rs`
- `code/front/src/page/public/article/index.rs`
- `code/front/src/page/public/article/search.rs`
- `code/front/src/page/public/article/update.rs`
- `code/front/src/page/public/article/version.rs`
- `code/front/src/page/public/article/version/create.rs`
- `code/front/src/page/public/article/version/detail.rs`
- `code/front/src/page/public/article/version/index.rs`
- `code/front/src/page/public/index.rs`
- `code/front/src/page/public/layout.rs`
- `code/front/src/page/time.rs`
- `code/front/src/pow.rs`
- `code/front/src/req.rs`
- `code/front/src/req/request.rs`
- `code/front/src/req/request/article.rs`
- `code/front/src/req/request/auth.rs`
- `code/front/src/req/request/comment.rs`
- `code/front/src/req/request/multipart.rs`
- `code/front/src/req/request/version.rs`
- `code/front/src/router.rs`
- `code/front/src/router/all.rs`
- `code/front/static/pow-worker.js`


## Improvements & refactors

> 🧠 **For the AI agent:** Propose concrete improvements for this unit: better types, dead-code removal, performance, accessibility, security, and tests. Mark each as `[keep-behavior]` so the rebuild stays functionally identical unless the user opts in.


## Redesign notes

> 🧠 **For the AI agent:** Map this unit onto the new architecture from `architecture/ARCHITECTURE.md`. Note where its files should live and which interfaces it exposes.


## Definition of done

- [ ] Every functional requirement is implemented and covered by a test.
- [ ] Every acceptance-criteria scenario passes (including the failure paths).
- [ ] Every operation this unit owns in `architecture/INTERFACES.md` responds correctly.
- [ ] Every entity it writes matches `architecture/DATA-MODEL.md` (fields, types, constraints).
- [ ] Every write is satisfiable against the schema: no required (NOT NULL, no-default) column or foreign key is left unfilled; anonymous/public operations write only to anonymous-capable entities (no owner FK).
- [ ] Every enum/domain value this unit uses is one of the members fully enumerated in `architecture/DATA-MODEL.md`.
- [ ] Every edge case & failure mode above is handled.
- [ ] `node scripts/analyze.mjs --check --out <out>` passes — no unresolved agent callouts or placeholders, and every reference resolves.
