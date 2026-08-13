# Interface surface

| Setting | Value |
| --- | --- |
| Mode | `redesign` |
| Level | `complex` |
| Fidelity | `describe` |
| TDD | `on` (build test-first) |
| Generated with | `reconstruct@2.17.0` |

All 31 routes below were verified against `code/back/src/api.rs` in the reference
tree. Every response uses the `{code, data, message}` envelope; `code` mirrors the
HTTP status, `data` is `null` on errors. Session auth means the `session-token`
request header. PoW means a server-issued challenge solved by MinRoot VDF
(proof-of-work, payload semantics per endpoint).

## Interface table

| Method | Path | Handler | Auth | Input | Output | Side effects / notes |
| --- | --- | --- | --- | --- | --- | --- |
| GET | `/challenge/read` | `api::authenticate::issue_challenge` | none | — | 200 `{data:{id: uuidv7, difficulty: 8192}}` | issues a single-use PoW challenge (TTL 300s), server-side accounting |
| GET | `/config/read` | `api::meta::read_config` | none | — | 200 `{data: {max_tags_per_article, max_comment_body_chars, max_version_note_chars, max_title_chars, max_summary_chars, max_pdf_size_bytes, max_text_field_bytes, download_token_ttl_seconds, ...limits}}` | runtime config served to the frontend |
| POST | `/email/read` | `api::authenticate::email_read` | optional session, branch-dependent | `EmailReadRequest` JSON: `{pow?, old_email_pow?, new_email_pow?}` | single-email (auth): 200 `{old_email_subject, new_email_subject}` style; dual-email (email change, session required): 200; deregister branch: 200 | one endpoint, three branches selected by presence of session and pow fields (see notes: auth vs deregister ambiguity) |
| POST | `/user/create` | `api::authenticate::redeem_token` | none (PoW only) | `TokenRequest` JSON `{pow: {challenge, solution, payload}}`; payload = whitespace-stripped emailed token (UUID) | 200 `{data: {session_token}}`; 400 invalid/expired token; 429 rate limited | user get-or-create by email hash, then session issuance |
| GET | `/session/read` | `api::authenticate::verify_session` | session-token | query: `id?`, `name?` (default false) | 200 `{data: {id?, name?}}` (only requested keys); 401 | |
| POST | `/session/delete` | `api::user::logout` | session-token + PoW | `LogoutRequest` `{pow}` (payload arbitrary nonce) | 200 `{data: {}, message: "deleted"}`; 400 PoW failures; 401 | revokes session from cache |
| GET | `/user/{id}/read` | `api::user::read_user` | session-token | path: user_id; query: `name?` (default true), `email_hash?` (default false for self, true for others) | self: 200 `{name?[, email_hash?]}`; other (admin `User::Read`): 200 `{id, name, email_hash}` | asymmetric email_hash defaults; self read swallows read errors |
| POST | `/user/{id}/update` | `api::user::update_user` | session-token | `UserUpdateRequest` union: `{name}` (admin rename) \| `{pow}` (self rename, payload = new name) \| `{pow, old_email_token, new_email_token}` (email change confirm) | admin rename 200 `{name}`; self rename 200 `{name}`; email confirm 200 `{session_token}` | |
| POST | `/user/{id}/delete` | `api::user::delete_user` | session-token | `UserDeleteRequest` `{mode: "transfer"\|"hard", pow}` | transfer 200 `{data: {}, message: "deleted"}`; hard (admin `User::Delete`) 200 `{user_id}` | transfer = soft delete to recycler; hard = recursive delete |
| GET | `/user/read` | `api::user::read_users` | session-token + `User::Read` (admin console) | query: `page?` (default 1, clamp 1..=10000), `limit?` (default 8, clamp 1..=200) | 200 `{user_list: [{id, name, email_hash}], has_next, total}` (id desc); 401; 403 | |
| GET | `/article/read` | `api::article::read_articles` | session-token (no permission gate) | query: `key_word?`, `ranges?`, `sort?`, `from?`, `to?`, `limit?`, `page?`; any search param present ⇒ search path, else plain list | search: `{article_list: [{id, title, author, time, hits: [{field, label, snippet}]}], page, total_pages, has_more, has_prev, total}`; list: `{article_list: [{id, title, summary}], ...}` | two data sources: agdb for list/count, seekstorm for search (see notes) |
| POST | `/article/create` | `api::article::create_article` | session-token + `Article::Create` | multipart: title (required ASCII 1..=200), summary (required ASCII 1..=2000), tags (required 1..=8 hashtags), version (required semver), note (required ASCII 1..=1024), file (required PDF ≤32MiB, %PDF- 1.x/2.x, %%EOF) | 201 `{data: {article_id, version_id}}`; 400 validation/duplicate | single transaction: article + version + tags + edge writes |
| GET | `/article/{id}/read` | `api::article::read_article` | session-token + `Article::Read` | path: article_id; query: `check_if_is_author?` | 200 `{id, author_id, author_name, title, summary, created_at, tags: [{id, name}]}` + `is_author?` when requested | |
| POST | `/article/{id}/update` | `api::article::update_article` | session-token + `Article::Update` | `UpdateArticleRequest` `{title, summary, tags}` (tags default ""); total body text ≤ 1MiB else 400 | 200 `{article_id}`; 400 duplicate title / tag errors | |
| POST | `/article/{id}/delete` | `api::article::delete_article` | session-token + `Article::Delete` | `DeleteBody` `{mode: "transfer"\|"hard"}` | 200 `{data: {article_id}, message: "deleted"}`; 400 missing/unsupported mode | transfer to recycler; hard recursive delete |
| GET | `/article/{id}/version/{version_id}/content/read` | `api::article::serve_public_pdf` | session-token + `Article::Read` | path: article_id, version_id; query: `download?:"1"\|"true"`, `token?` | no query: 200 application/pdf inline; download=1: mints URL token (see notes: broken); token=: consume single-use token | PDF download mint+consume is internally inconsistent (see notes) |
| POST | `/article/{id}/version/create` | `api::version::create_version` | session-token + `Version::Create` | multipart: version (semver, strictly greater than max), note (ASCII 1..=1024), file (valid PDF ≤32MiB) | 201 `{data: {version_id}}`; 400 version not greater / duplicate content hash | content-hash dedupe |
| GET | `/article/{id}/version/read` | `api::version::read_versions` | session-token (no permission gate) | query: `page?`, `limit?` (clamped) | 200 `{version_list: [{id, version, created_at}], page, total, has_next}` (newest first) | session-only, unlike gated single-version read (see notes) |
| GET | `/version/{id}/read` | `api::version::read_version` | session-token + `Version::Read` | path: version_id; query: `article_id?` (must match), `check_if_is_author?` | 200 `{id, version, created_at, note}` + `is_author?`; 401; 403; 404 | |
| POST | `/version/{id}/update` | `api::version::update_version` | session-token + `Version::Update` | `UpdateVersionNoteRequest` `{note (ASCII 1..=1024)}` | 200 `{version_id}`; 400; 401; 403; 404 | |
| POST | `/version/{id}/delete` | `api::version::delete_version` | session-token + `Version::Delete` | `DeleteBody` `{mode}` — only `"hard"` accepted | 200 `{data: {version_id}, message: "deleted"}`; 400 | hard delete: version + comment tree + PDF |
| POST | `/version/{id}/comments/create` | `api::comment::create_comment` | session-token + `Comment::Create` | `CreateCommentRequest` `{content (ASCII 1..=1024)}` | 201 `{data: {comment_id}}`; 400; 401; 403; 404 | |
| POST | `/comments/{id}/replies/create` | `api::comment::create_reply` | session-token + `Comment::Create` | `CreateCommentRequest` `{content}` | 201 `{data: {comment_id}}`; 400 thread too deep (depth ≤ 64) | |
| GET | `/version/{id}/comments/read` | `api::comment::read_comments` | session-token + `Comment::Read` on the version | query: `page?`, `limit?`, `check_if_is_author?` | 200 `{comments: [{id, content, user_id, parent_id, created_at, user_name}], has_next, total}` + version-level `is_author` | no per-comment is_author (see notes); top-level paging only |
| POST | `/comment/{id}/update` | `api::comment::update_comment` | session-token + `Comment::Update` | `CreateCommentRequest` `{content}` | 200 `{comment_id}`; 400; 401; 403; 404 | |
| POST | `/comment/{id}/delete` | `api::comment::delete_comment` | session-token + `Comment::Delete` | `DeleteBody` `{mode: "transfer"\|"hard"}` | 200 `{data: {comment_id}, message: "deleted"}`; 400 missing/unsupported mode | |
| POST | `/role/create` | `api::role::create_role` | session-token + `Role::Manage` | `CreateRoleRequest` `{name (trimmed 1..=64 ascii alnum/-/_, lowercased)}` | 201 `{data: {name}}` — also when the role already exists (idempotent, see notes) | |
| GET | `/role/read` | `api::role::read_roles` | session-token + `Role::Manage` | query: `page?`, `limit?` (clamped) | 200 `{role_list: [{name, permissions, scopes, member_count}], has_next, total}` (name asc; member_count always 0, see notes) | |
| GET | `/role/{name}/read` | `api::role::read_role` | session-token + `Role::Manage` | path: name | 200 `{name, permissions, scopes, members: [user ids sorted]}`; 401; 403; 404 | |
| POST | `/role/{name}/update` | `api::role::update_role` | session-token + `Role::Manage` | `RoleUpdateRequest` `{permissions?: {add, remove}, tags?: {add, remove}, users?: {add, remove}}` (all optional) | 200 `{name}`; 400 required-role protection; 401; 403; 404 | single transaction; `member` role protected from destructive changes |
| POST | `/role/{name}/delete` | `api::role::delete_role` | session-token + `Role::Manage` | `DeleteBody` `{mode}` — only `"hard"` accepted | 200 `{data: {name}, message: "deleted"}`; 400 | guards only admin/recycler (see notes: member deletable) |

## Realtime / WebSocket

None.

## Auth / middleware

- Session: `session-token` header; sessions cached in moka (TTL 8000s), reverse-indexed by user.
- PoW: MinRoot VDF, difficulty 8192 iterations, challenge TTL 300s, single-use.
- Authorization: Cedar (`schema.cedar` actions, `policy.cedar` policies), assembled per request by `authorization::entity_store` + `gate::authorize`.
- Rate limiting: enforced by the pingap reverse proxy (`conf/proxy/plugins.toml`), not the backend.
