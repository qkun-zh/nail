# ADR-0004: Interface-layer CRUD purity audit

## Status

Accepted; all deviations resolved (2026-08-15). See "Resolution" below.

## Context

The project constitution (README §5.2) mandates a "backend CRUD pure"
philosophy at the interface layer:

- Every backend resource is operated on with exactly
  `create`/`read`/`update`/`delete`.
- Collection reads are `read` (never `list`), paginated through query
  parameters.
- **search is a `read`** — a read with query parameters, not a fifth verb.
- Wire-flow vocabulary (e.g. `intent=authenticate|change_email|deregister`)
  never appears as a backend identifier; name the node op instead
  (`create_user`, `update_user_email`, `delete_user`).
- `interface` is the strictest layer: one `<verb>_<resource>` handler per
  route.

This ADR records a line-by-line audit of `code/back/src/interface/**` against
that philosophy, so the principle has an explicit, reviewable home and future
agents know what "compliant" looks like.

## Audit result: largely compliant on domain resources

The domain resources fully honor the closed-verb algebra and the
search-is-a-read rule:

- **Search is a read.** `/article/read` (`article::read_articles`) takes
  `ArticleSearchParams`; a search query is the same collection-read with query
  parameters populated. There is no separate `/article/search` endpoint.
  Full-text search lives only in `logic/search.rs` /
  `repository/search` (infrastructure vocabulary — `search_articles`,
  `sync_*`), exactly as the philosophy prescribes.
- **Collection reads are `read`.** `read_articles`, `read_users`, `read_roles`,
  `read_versions`, `read_comments`, `read_comment_children` — no `list`.
- **One verb per resource.** user, article, version, comment, role, session all
  expose exactly create/read/update/delete.
- **Delete mode is a payload field, not a verb.** `DeleteBody { mode:
  Hard|Transfer }` — still a single `delete` action.
- **Sub-resource verbs stay CRUD.** `create_reply` / `read_comment_children`
  name a nested *resource* (reply), not a new verb; the verb remains create/read.

## Deviations (not fully practicing the philosophy)

Three places fall short. Two are non-resource system flows; one is a hidden
side effect inside a read.

1. **`/challenge/read` → `challenge::create_challenge` → logic
   `create_challenge` (route verb disagreement).** The route path claims the
   verb is `read`, but the handler and logic name it `create`. A challenge is
   *issued/generated*, not a persisted CRUD entity. Whichever resolution is
   chosen, the three layers must agree — today the strictest layer contradicts
   itself (router.rs:19 vs challenge.rs:9 vs logic/challenge.rs:7).

2. **`/email/read` POST → `read_email` is a side-effecting flow dispatch, not a
   read.** `logic/email.rs::read_email` branches on `intent` =
   `authenticate|change_email|deregister` and *sends emails and triggers state
   transitions* (send_create_user_email, send_update_user_email,
   send_delete_user_email). This violates the philosophy twice:
   - The verb `read` is wrong — the operation mutates state / dispatches mail.
     The three branches are really "begin the create-user / change-email /
     deregister flow", i.e. flow verbs.
   - The query parameter `intent` (interface/email.rs:13) is the exact
     wire-flow vocabulary §5.2 forbids as a backend identifier.

   The underlying node operations already exist as proper endpoints
   (`/user/create`, `/user/{id}/update` for email change, `/user/{id}/delete`),
   so `/email/read` is a flow-orchestration endpoint the interface should not
   carry under a `read` verb.

3. **`/content/read` hides a create inside a read (minor).** `read_content`
   (content.rs:26) has a `download` branch that calls
   `logic::download::mint_download_token` — minting a token is a create/side
   effect, yet it lives under a read route switched by a query parameter. The
   response is still the content, so this is the least severe, but it is a
   mutation hiding in a read.

## Decision

Record the audit as the canonical reading of §5.2. The audit establishes the
rule of thumb used everywhere below:

- **Interface verbs stay a closed set** (`create`/`read`/`update`/`delete`);
  any operation that changes state must not be named `read` and must not hide
  behind `intent`-style flow parameters.
- **A read that has side effects is not a read** — name the effect or split it
  into the underlying node operation.
- **The route path, the handler name, and the logic entry point must use the
  same verb**; the interface layer is the strictest, so any disagreement is a
  defect.
- **Rich vocabulary is legitimate for sub-resources and views** (`reply`,
  `children`, `download` as a *projection*), but never as a new top-level verb.

## Consequences

- Domain-resource endpoints already comply; no change needed there.
- `/challenge/read` needs a single consistent verb (likely `create_challenge`
  everywhere, or rename the route to `challenge/create`).
- `/email/read` should be decomposed or renamed to node operations; `intent`
  must not appear as a backend identifier. This is the largest remaining
  refactor and is deferred pending owner decision (it touches the frontend
  caller and the common request/response types).
- `/content/read`'s `download` token-mint is acceptable as a projection but is
  noted so it is not propagated.
- This ADR is the reference for the §5.2 enforcement depth; it does not change
  the constitution text.

## Resolution (applied 2026-08-15)

The three deviations were fixed after settling the design with the owner:

1. **Challenge** — `GET /challenge/read` → `POST /challenge/create`; handler and
   logic were already `create_challenge`, so only the route verb and HTTP method
   changed. Minting a fresh nonce is a non-idempotent create, hence POST.
2. **Email** — `/email/read` → `POST /token/create`. The resource is now a
   `token`; the `intent` query vocabulary is replaced by a closed `purpose`
   body field naming the node op (`create_user | update_user_email |
   delete_user`). `EmailReadIntent`→`TokenPurpose`, `EmailReadRequest`→
   `CreateTokenRequest`, `EmailReadView`→`CreateTokenView`; `parse_intent`
   removed; `interface/email.rs` → `interface/token.rs`.
3. **Content** — the dead no-token branch of `read_content` was removed; the
   endpoint now requires a `token` (or `?download=1` to mint one).
   `resolve_version_pdf_path` remains (used by `mint_download_token`).

The audit's rule of thumb stands unchanged and is the reference for §5.2.

## Follow-up (resolved)

- [x] Align `/challenge/read` route + handler + logic verb.
- [x] `/email/read` → `token/create` with `purpose`; `intent` removed from the
      backend identifier surface.
- [x] Remove the dead no-token `/content/read` branch.
