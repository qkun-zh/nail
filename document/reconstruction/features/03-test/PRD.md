# Test

> Unit `03-test` · kind: feature

## Summary

Groups 70 file(s).

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

- `test/README.md`
- `test/end_to_end/browser/account.rs`
- `test/end_to_end/browser/article_lifecycle.rs`
- `test/end_to_end/browser/article_pages.rs`
- `test/end_to_end/browser/author_gate.rs`
- `test/end_to_end/browser/comments.rs`
- `test/end_to_end/browser/context.rs`
- `test/end_to_end/browser/login_workflow.rs`
- `test/end_to_end/browser/mod.rs`
- `test/end_to_end/browser/search_full.rs`
- `test/end_to_end/http/context.rs`
- `test/end_to_end/http/mod.rs`
- `test/end_to_end/http/search.rs`
- `test/end_to_end/http/smtp_sink.rs`
- `test/end_to_end/mod.rs`
- `test/unit/back/configuration/mirror_consistency.rs`
- `test/unit/back/configuration/validation.rs`
- `test/unit/back/context.rs`
- `test/unit/back/context_fixtures.rs`
- `test/unit/back/harness.rs`
- `test/unit/back/http/article.rs`
- `test/unit/back/http/article_delete.rs`
- `test/unit/back/http/article_pdf.rs`
- `test/unit/back/http/article_search.rs`
- `test/unit/back/http/article_update.rs`
- `test/unit/back/http/authenticate.rs`
- `test/unit/back/http/author.rs`
- `test/unit/back/http/comment.rs`
- `test/unit/back/http/comment_delete.rs`
- `test/unit/back/http/contract.rs`
- `test/unit/back/http/download.rs`
- `test/unit/back/http/email.rs`
- `test/unit/back/http/meta.rs`
- `test/unit/back/http/proxy_and_attachment.rs`
- `test/unit/back/http/user.rs`
- `test/unit/back/http/version.rs`
- `test/unit/back/logic/article.rs`
- `test/unit/back/logic/article_search.rs`
- `test/unit/back/logic/authenticate.rs`
- `test/unit/back/logic/author.rs`
- `test/unit/back/logic/comment.rs`
- `test/unit/back/logic/concurrency.rs`
- `test/unit/back/logic/concurrency/util.rs`
- `test/unit/back/logic/download.rs`
- `test/unit/back/logic/email.rs`
- `test/unit/back/logic/email_service.rs`
- `test/unit/back/logic/user.rs`
- `test/unit/back/logic/version.rs`
- `test/unit/back/pdf/validation.rs`
- `test/unit/back/repository/article.rs`
- `test/unit/back/repository/authorization.rs`
- `test/unit/back/repository/comment.rs`
- `test/unit/back/repository/db.rs`
- `test/unit/back/repository/schema.rs`
- `test/unit/back/repository/search.rs`
- `test/unit/back/repository/tag.rs`
- `test/unit/back/repository/token.rs`
- `test/unit/back/repository/token_ttl.rs`
- `test/unit/back/repository/transfer.rs`
- `test/unit/back/repository/types.rs`
- `test/unit/back/repository/user.rs`
- `test/unit/common/hash/tests.rs`
- `test/unit/common/name/tests.rs`
- `test/unit/common/pow/tests.rs`
- `test/unit/common/request/tests.rs`
- `test/unit/common/response/tests.rs`
- `test/unit/common/search/tests.rs`
- `test/unit/common/tag/tests.rs`
- `test/unit/common/text/tests.rs`
- `test/unit/common/time/tests.rs`


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
