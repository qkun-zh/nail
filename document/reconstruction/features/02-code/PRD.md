# Code (02-code)

## Summary

The nail platform: a proof-of-work-gated, session-authenticated article library. Authors publish PDF-backed articles with semver versions and a bounded-depth threaded comment tree; admins manage users and roles; deleted or deregistered content is recycled to the least-loaded recycler account; an embedded search index serves full-text queries. This unit is the entire `code/` workspace: the axum backend (interface, logic, agdb repository, Cedar authorization, infrastructure), the shared `common` crate, and the Leptos CSR frontend.

## Context & goal

- Depends on: `01-project-setup` (workspace Cargo.tomls), the configuration tree (`conf/back/server.toml|smtp.toml|email.toml`, `conf/front/config.toml`), and the pingap proxy (rate limiting and `client_max_body_size` live there, not in this unit).
- Depends on this unit: nothing else in the tree; it is the application.
- Goal: a visitor can authenticate (email + one-time token, each step PoW-gated), then read/search articles, publish and version them as PDFs, comment, manage name/email/account, and — as admin or recycler — administer users, roles, and recycled assets. Every response is the `{code, data, message}` envelope; every mutation is permission-gated by Cedar; every human-gated action is PoW-gated; all IDs are UUIDv7; all hashing is ascon-family.

## User stories

- As an anonymous visitor, I can fetch a challenge and send my email with its PoW proof, then paste the emailed token with a new proof, so I can create an account or log in without a password.
- As an anonymous visitor, I can read the runtime limits (`GET /config/read`) without any session.
- As an authenticated user, I can verify my session and read my id and display name.
- As an authenticated user, I can list and search articles and read any article's detail, its version list, a version's detail, and its PDF content.
- As an authenticated user, I can publish an article (title, summary, hashtags, semver version, note, valid PDF) and add strictly-greater versions with new PDFs.
- As an author, I can update my article's title/summary/tags, update any version's note, delete an article (transfer or hard), and hard-delete a version.
- As an authenticated user, I can comment on a version, reply to comments (depth-bounded), edit my comment content, and delete my comments (transfer or hard).
- As an authenticated user, I can download a version's PDF.
- As an authenticated user, I can set my display name, change my email (two-token confirmation), log out, and deregister my account (assets transfer to a recycler).
- As an author, I can check whether I am allowed to modify a given article/version/comment (`is_author`).
- As an admin, I can read any user, list all users, rename any user, hard-delete users, and manage roles (create/read/list/update/delete; grant/revoke permissions, tag scopes, members).
- As a role-scoped member, my granted actions apply only to articles whose tags intersect my role's tag scopes (a scope-less role is global); the seeded `member` role can create articles and comments.
- As a recycler, deleted articles/comments and deregistered accounts' assets land on the least-loaded recycler account.
- As an operator, the backend seeds permissions from `schema.cedar`, seeds user zero (admin + recycler + member), rebuilds the search index at startup, cleans leftover upload temp files, and rotates logs by minute with retention pruning.
- As a user with an expired session, the frontend clears my stored token and prompts me to authenticate again.

## Functional requirements

### A. Boot, config, infrastructure
- FR-1 [confirmed] Backend boots by loading `server.toml`, `smtp.toml`, `email.toml` (env `CONF_DIR` overrides; else walk up to `conf/back`). Config is validated at startup (see edge cases); failures log to `startup-errors.log` and exit code 1.
- FR-2 [confirmed] Database is agdb, memory or mapped file per `db_path` (`memory|mem|:memory:|in-memory` selects in-memory).
- FR-3 [confirmed] Graph init creates indexes for `email_address_hash`, `name`, `title`, `content_hash`, `tag_name`, `role_name`, `permission_name`; seeds permission nodes from `schema.cedar` actions; creates/finds user zero by hashed `user_zero_email` and holds `admin`, `recycler`, `member` roles (admin granted all 16 permissions, member granted `Article::Create` + `Comment::Create`).
- FR-4 [confirmed] SeekStorm index opened or created (schema: id, title w=3.0, summary w=1.0, author w=2.0, note w=1.0, tag JSON, comment JSON, ts Timestamp; BM25f, UnicodeAlphanumericFolded, Snappy, mmap); the whole index is rebuilt from agdb at startup.
- FR-5 [confirmed] PDF storage dir and its `.tmp` dir are created at startup; leftover `.tmp` files are deleted; uploads stream to `<pdf_storage_path>/.tmp/<uuidv7>.pdf` and are renamed to `<storage>/<hh>/<mm>/<hash>.pdf` on success (path from content hash: 2+2 hex chars + hash).
- FR-6 [confirmed] Logging: tracing-subscriber to per-minute files `log/back/<day>/<hour>/<day>_<hour-minute>.log` via a writer thread; startup + periodic prune by retention days and ring size.
- FR-7 [confirmed] Graceful shutdown on Ctrl-C / SIGTERM; search index closed on exit. Every API response is `{code, data, message}` with code = HTTP status; internal errors always surface message "internal server error".
- FR-8 [confirmed] Email service: SMTP (STARTTLS or plaintext local relay), per-recipient cooldown `email_cooldown_seconds` (60) before send; command timeout + whole-call wall-clock timeout; body = one-time token UUID, subject = fresh UUIDv7.

### B. Auth & PoW
- FR-9 [confirmed] `GET /challenge/read` (no session) returns `{id: uuidv7, difficulty}`; the challenge is cached (TTL 300 s) and single-use. Difficulty must equal server config `pow_difficulty_iterations`.
- FR-10 [confirmed] PoW: client proves by MinRoot VDF over an ascon-CXOF-derived input (solution = hex(48-byte output ‖ 48-byte proof)); server verifies solution length (192 hex chars, 96 bytes), payload ≤ 4096 bytes, solution hex ≤ 4096 chars, and exact difficulty match.
- FR-11 [confirmed] Email auth request (`POST /email/read`, no session branch): email is normalized (trim, lowercase), must be ≤ 254 chars, parse as email, and its domain must be in `allowed_domains`; PoW payload must be the email; then an auth email is sent and the token cached keyed by token hash (TTL 8000 s) with reverse index by email hash.
- FR-12 [confirmed] Token exchange (`POST /user/create`): PoW payload is the emailed token (whitespace-stripped, must parse as UUID); token consumed; user found or created by email hash (default name = id without dashes); `member` role held; a session token is created (cache, TTL 8000 s, reverse index by user) and returned. On DB failure the auth token is re-created so the user can retry.
- FR-13 [confirmed] Session validation: token normalized (whitespace stripped, UUID); lookup by token hash; else 401 "invalid session".
- FR-14 [confirmed] `GET /session/read` returns only requested `id`/`name` flags.
- FR-15 [confirmed] Logout (`POST /session/delete`) requires a valid session AND a fresh PoW (payload arbitrary, e.g. random nonce); deletes the session token.
- FR-16 [confirmed] Every PoW verifies challenge issuance/expiry/one-time consumption first; failures are 400 "challenge not issued, expired, or already used" / "PoW verification failed".

### C. User management
- FR-17 [confirmed] Self profile read (`GET /user/{id}/read` with id = own id): name by default, email_hash only if explicitly requested; other ids require `User::Read` (admin console) and return id/name/email_hash filtered by `name`/`email_hash` params.
- FR-18 [confirmed] Name update: self path requires PoW whose payload is the new name, validated by `common::name::validate_name` (trimmed, 1..=32 chars, ascii alnum / `-` / `_`); uniqueness enforced on `name` index (400 "name already taken"); search index re-synced for the user's articles (best-effort).
- FR-19 [confirmed] Admin rename (`name` field in body, no PoW) requires `User::Update`; same validation and uniqueness.
- FR-20 [confirmed] Admin user list (`GET /user/read`) requires `User::Read`; `page`/`limit` clamped (limit 1..=200, page 1..=10000); sorted by id desc; returns `{user_list, has_next, total}`.
- FR-21 [confirmed] Email update — step 1 (`POST /email/read` with both `old_email_pow` and `new_email_pow`): requires session; old must hash to the account's email hash; both domains allowed; old ≠ new; new email not already used by another account; two emails sent; an email-update token row cached by user (token hashes stored).
- FR-22 [confirmed] Email update — step 2 (`POST /user/{id}/update` with `old_email_token` + `new_email_token` + PoW): PoW payload must equal `"<old_token>\n<new_token>"` (canonical or raw); both tokens must parse as UUID and differ; cached hashes must match; new email still free; email hash updated; all sessions invalidated; auth/deregister tokens for the old email purged; a NEW session token is issued and returned.
- FR-23 [confirmed] Deregister — step 1 (`POST /email/read` with a valid session and `pow` whose payload is the account email): email must hash-match; a confirmation email is sent; deregister token cached (reverse by user).
- FR-24 [confirmed] Deregister — step 2 (`POST /user/{id}/delete` mode `transfer`): requires PoW with the emailed token; token must match the session user (400 "deregister token does not match your account"); all articles and comments of the user are re-pointed to the least-loaded recycler; the user node is removed; all sessions/tokens purged; search re-synced per article.
- FR-25 [confirmed] Hard-delete user (`POST /user/{id}/delete` mode `hard`): requires `User::Delete` (admin); cascades: all the user's articles (with their versions and all comments on those versions), the user's comment trees, then the user node; orphaned PDF files removed; index rebuilt (best-effort).
- FR-26 [confirmed] Delete-mode contract: user delete accepts only `transfer` or `hard`; any other/missing mode is 400 "missing or unsupported delete mode (expected \"transfer\" or \"hard\")".

### D. Articles & versions
- FR-27 [confirmed] `POST /article/create` (multipart: title, summary, tags, version, note, file) requires session + `Article::Create` permission. Validations: title non-empty ASCII 1..=200 (no newline); summary non-empty ASCII 1..=2000 (newlines ok); tags parsed by `parse_hashtag_tags` (1..=8, each `#` + 1..=31 ascii alnum/`-`/`_`, no `#` inside), at least one tag; version = semver, canonicalized; note non-empty ASCII 1..=1024 (newlines ok); PDF stream-validated (≥10 bytes, `%PDF-`, version `1.x`/`2.x`, trailing `%%EOF` within last 1024 bytes, ≤ `max_pdf_size_bytes`); title and content hash must be unique (400 "title already exists" / "identical PDF already exists (version v of \"title\")"); author must exist. Returns 201 `{article_id, version_id}`. Writes: version node, article node (visibility `public`, latest_version_id), edges user_to_article / article_to_version / article_to_tag (tags get-or-create), PDF file; search index synced best-effort.
- FR-28 [confirmed] `POST /article/{id}/update` requires session + `Article::Update` (owner or permission) on the article; same title/summary/tags validation; tag edges reconciled (new added, stale removed, orphan tags with zero incoming edges deleted); title uniqueness against other articles; index re-synced. Returns `{article_id}`.
- FR-29 [confirmed] `POST /article/{id}/delete` requires `Article::Delete`. Mode `transfer`: owner edge re-pointed to least-loaded recycler (404 "article not found" if missing; 500 "no recycler available" if none), index re-synced. Mode `hard`: cascade-delete article + all versions + all comments on them, collect content hashes, remove orphaned PDF files, rebuild index. Returns `{article_id}` "deleted".
- FR-30 [confirmed] `GET /article/read` requires only a session. If any of `key_word|ranges|sort|from|to` is present → search path (see G); else plain listing: agdb articles ordered by id desc, paginated (limit 1..=200 default 8, page 1..=10000), enriched with author id/name, tags, latest_version + latest_version_id; returns `{article_list, page, total, total_pages, has_next, has_prev, truncated}` (truncated = total_pages > max_search_pages).
- FR-31 [confirmed] `GET /article/{id}/read` requires session + `Article::Read`; returns `{id, author_id, author_name, title, summary, created_at, tags:[{id,name}]}` (+ `is_author` when `check_if_is_author=true`); 404 "article not found". created_at derived from the article id's uuidv7 timestamp (0 if not v7).
- FR-32 [confirmed] `POST /article/{id}/version/create` (multipart version, note, file) requires session + `Version::Create` on the article; version must parse as semver and be strictly greater than the max existing version (400 "new version must be strictly greater than the latest version"); content hash unique (400 "identical PDF already exists"); note ASCII 1..=1024; article must exist (404). Writes version node, article_to_version edge, updates latest_version_id if the new id sorts higher; PDF placed; index synced. Returns 201 `{version_id}`.
- FR-33 [confirmed] `GET /article/{id}/version/read` requires only a session; returns `{version_list:[{id, version, created_at}], page, total, has_next}` (versions sorted by id desc, i.e. newest first; created_at from version id uuidv7).
- FR-34 [confirmed] `GET /version/{id}/read` requires session + `Version::Read`; optional `article_id` query — if given and the version does not belong to that article → 404 "version not found"; returns `{id, version, created_at, note}` (+`is_author` when requested).
- FR-35 [confirmed] `POST /version/{id}/update` requires session + `Version::Update`; body `{note}`; note ASCII 1..=1024; returns `{version_id}`.
- FR-36 [confirmed] `POST /version/{id}/delete` accepts only `mode:"hard"` (400 otherwise) and requires `Version::Delete`; removes the version, its comment trees, refresh article latest_version_id (max id of remaining), cleanup orphan PDFs; returns `{version_id}` "deleted".
- FR-37 [confirmed] PDF content route `GET /article/{id}/version/{version_id}/content/read`: requires session + `Article::Read`; no query params → serves the PDF file (application/pdf, `Content-Disposition: attachment; filename=<sanitized>`; filename chars limited to ascii alnum `-` `_` `.`, fallback `article.pdf`); 404 "PDF file not found"; 500 on IO failure. `download=1|true` → mints a download token and returns `{url: "/api/article/{id}/version/{version_id}/content/read?version_id={version_id}"}`; `version_id`/`token` query → consumes a download token (see notes: wiring broken in source).
- FR-38 [confirmed] Download token: mint binds token→(version_id, user_id) with TTL 60 s; consume requires the same session user (400 "download token is bound to another account"), article still readable, token single-use (400 "invalid or expired download token").

### E. Comments
- FR-39 [confirmed] `POST /version/{id}/comments/create` requires session + `Comment::Create`; content ASCII 1..=1024; version must exist (404 "comment target not found (the version may have been removed)"); writes comment node + user_to_comment + comment_to_version edges; index synced (best-effort). Returns 201 `{comment_id}`.
- FR-40 [confirmed] `POST /comments/{id}/replies/create` requires session + `Comment::Create`; parent must exist (404 "reply target not found (the parent comment may have been removed)"); parent-chain depth must be < `max_comment_tree_depth` (64) → 400 "comment thread too deep (max 64 reply layers)"; writes comment + user_to_comment + comment_to_comment edges; index synced.
- FR-41 [confirmed] `GET /version/{id}/comments/read` requires session + `Comment::Read` on the version; version must exist (404); pages by TOP-LEVEL comments (page/limit clamped as usual); each page's comment tree (replies, depth-bounded) is included; response `{comments:[{id, content, user_id, parent_id, created_at, user_name}], has_next, total}` (+ top-level `is_author` when `check_if_is_author=true`). created_at from comment id uuidv7; user_name from a batch user lookup (empty name if lookup fails).
- FR-42 [confirmed] `POST /comment/{id}/update` requires session + `Comment::Update`; content re-validated; returns `{comment_id}`.
- FR-43 [confirmed] `POST /comment/{id}/delete` requires `Comment::Delete`; mode `transfer` → owner edge re-pointed to recycler (404 "comment not found", 500 "no recycler available"); mode `hard` → delete comment + its reply subtree. Returns `{comment_id}` "deleted".

### F. Search
- FR-44 [confirmed] Search request (any of q/ranges/sort/from/to): q trimmed, ≤ `max_search_query_chars` (512) chars (400 "query string too long (max 512 chars)"); ranges comma-list of `title|summary|author|comment|note|tag`, unknown value → 400 "invalid range: X", empty → all six; sort comma-list of `field:asc|desc` with field ∈ `time|title|author` (default `time:desc`, `title:asc`, `author:asc`), malformed → 400; from/to epoch seconds, from > to → 400 "from cannot be later than to"; page 1..=`max_search_pages` (1024), limit 1..=200.
- FR-45 [confirmed] Query executes against SeekStorm (lexical intersection, BM25f); `time` window is a ts facet filter; hits carry `{field, label, snippet}` where label is the Chinese range label (标题/摘要/作者/评论/版本说明/标签) and snippets contain `<mark>` markup; empty q returns all documents in window; total = index count for q, else agdb count in time window.
- FR-46 [confirmed] Search response: `{article_list:[{id, title, author, time, hits}], page, total, total_pages, has_next, has_prev, truncated}`; total_pages capped at max_search_pages; page beyond total → empty list with has_prev per page>1; `time` = RFC3339 with fixed +08:00 offset.

### G. Roles & authorization
- FR-47 [confirmed] `POST /role/create` requires `Role::Manage`; name trimmed, 1..=64 chars, ascii alnum/`-`/`_` (400 "invalid role name"); create is idempotent (existing role returns 201 with the same name); writes role node.
- FR-48 [confirmed] `GET /role/read` (page/limit clamped) requires `Role::Manage`; roles sorted by name; returns `{role_list:[{name, permissions, scopes, member_count}], has_next, total}` (member_count is always 0 in source — see notes).
- FR-49 [confirmed] `GET /role/{name}/read` requires `Role::Manage`; returns `{name, permissions, scopes, members:[user ids sorted]}`; 404 "role not found".
- FR-50 [confirmed] `POST /role/{name}/update` requires `Role::Manage`; body `{permissions?:{add,remove}, tags?:{add,remove}, users?:{add,remove}}`; for REQUIRED_ROLES (admin/recycler/member) any remove-side change is rejected 400 ("role X is a required role and cannot be modified destructively"); grants/revokes permission edges (permission must be a seeded node), applies/removes tag scope edges (tags get-or-create), holds/unholds users (user must exist).
- FR-51 [confirmed] `POST /role/{name}/delete` accepts only `mode:"hard"` and requires `Role::Manage`; deleting `admin` or `recycler` is rejected 400 ("role X is a required role and cannot be deleted") — note `member` is NOT guarded here (inconsistent with FR-50).
- FR-52 [confirmed] Cedar authorization: schema.cedar declares 8 entity types and 16 actions; policy.cedar (5 policies) — (1) owner bypass: resource.owner == principal allows Read/Update/Delete/Comment::Delete/Version::Create; (2) `Visibility::"public"` allows Read actions; (3) role-permission with scope: `principal in action && (principal.global_role || principal.scopes.containsAny(resource.required_scopes))`; (4) admin-console actions (`User::Read/Update/Delete`, `Role::Manage`) require resource == `System::"admin-console"`; (5) any user in `Role::"admin"` is allowed everything.
- FR-53 [confirmed] Assembly: principal = User entity with `global_role` (any held role with empty scopes) and `scopes` (union of role tags), parents = held roles, roles' parents = granted action entities; resource = Article/Version/Comment entity chain with `owner`, `visibility` (default `private` when missing), `required_scopes` (article tags), parents chain (comment→version→article); `authorize` → 403 "you are denied"; missing resource → 404; `authorize_or` maps NotFound to the caller's message; `authorize_create` evaluates against a synthetic `Article::"__create__"` resource.
- FR-54 [confirmed] `is_author` check: exactly one of article_id/version_id/comment_id (else 400); article → `Article::Update` on the article; version → `Article::Update` on the version resource (owner semantics); comment → `Comment::Delete` on the comment (unreachable via API — see notes); returns bool.
- FR-55 [confirmed] Recycler selection: among users holding `recycler` role, pick the one with the fewest article+comment ownership edges (tie → larger user id); exclude the transferring author during account transfer; none → 500 "no recycler available".

### H. Frontend
- FR-56 [confirmed] Frontend boots: `main.rs` provides the notification system and the limits signal, then mounts the router; `conf/front/config.toml` (`api_base_url`, empty = same-origin) is embedded at compile time and panics on invalid scheme.
- FR-57 [confirmed] Runtime limits are fetched from `/api/config/read` into a signal with compile-time defaults until the fetch completes; per-field zero values fall back to defaults.
- FR-58 [confirmed] `request` layer owns all HTTP: 30 s abort timeout per call; session token from localStorage key `session_token`; a 401 on an authenticated call clears the token and marks the session invalid; envelope unwrapped and non-2xx code/message surfaced as an error; URLs are `api_base_url + "/api" + path` with `encodeURIComponent` on path segments.
- FR-59 [confirmed] PoW runs in-wasm on the main thread (`common::pow::prove`); each gated action fetches a fresh challenge first. (Worker leftover — see notes.)
- FR-60 [confirmed] Router: `/` index (session gate with links), `/public` (index, article index/search/create, article detail/update/delete, version list/create/detail, comment sub-paths `comment` / `comment/{id}` / `comment/{id}/delete`), `/private` (index, authenticate, name, name/update, email, email/update, logout, deregister), fallback 404.
- FR-61 [confirmed] Session gate: verifies once via `GET /session/read`; states checking/authenticated/anon ("who are you?" prompt with link to `/private/authenticate`); `/private/authenticate` always renders even when unauthenticated.
- FR-62 [confirmed] Author gate (`use_author_gate`): re-checks `is_author` whenever the target id changes; stale responses dropped by sequence guard; denial renders "you are denied!"; errors toast.
- FR-63 [confirmed] Pages validate inputs client-side with the same rules as the backend (name, ascii text, tags, PDF sniffing via MIME/name, file size) before submitting.
- FR-64 [confirmed] Notifications: toasts (error 5 s, success 3 s, info), per-toast countdown, dismiss button, history (cap 100) toggle.
- FR-65 [confirmed] Download: minted URL must be same-origin (else refuse to send the token); response bytes are saved via blob + anchor click; filename from Content-Disposition (fallback `article.pdf`).
- FR-66 [confirmed] Forms persist drafts into URL query params (replace navigation) so refreshes keep input.

## Interfaces & data

- Route table: every one of the 31 HTTP operations is proposed in `architecture/INTERFACES.md` (method · path · kind · auth · input · output · side-effects), including all error envelopes. This unit consumes the 31 rows; the frontend reaches 21 of them (all except `/user/read`, user hard-delete, `/role/*`, `/version/{id}/update`, `/version/{id}/delete`, `/comment/{id}/update`).
- Data model: `architecture/DATA-MODEL.md` — agdb node entities user/article/version/comment/tag/role/permission with the 9 edge types (user_to_article, article_to_version, user_to_comment, comment_to_version, comment_to_comment, article_to_tag, user_hold_role, role_grant_permission, role_apply_tag); the 8 Cedar entity types; and the common DTOs (Challenge, Pow, ProveInput, ResponseEnvelope, request payloads, ArticleSearchParams, SearchHit/SearchArticleItem/SearchPage, RoleListItem, token-cache entries).
- Envelope: `{code: u16, data: T?, message: String}`; success messages "ok"/"created"/"deleted"; errors carry `data: null`; `LogicError::Internal` always renders as message "internal server error" and is logged server-side.
- Write contract (satisfiability): every mutation is session-bound; the actor id comes from the session token lookup, so no owner FK is ever anonymous. The only anonymous writes are (a) `/user/create` — creates a user node from a PoW-verified, email-hash-bound token (no owner FK) and holds the member role; (b) token/challenge cache entries. Required fields on create: article (title, summary, visibility=public default, latest_version_id), version (version_number, content_hash, note) — all from the validated body or generated ids; tag names from body; comment content from body; role/permission names from body or the Cedar schema seed. PDF files are content-addressed by ascon hash; paths derived from the hash.
- Enum usage: deletion modes `transfer`/`hard` (version/role: `hard` only) and search sort directions `asc`/`desc` are the only stringly-typed enums accepted in request bodies — all others are validated server-side against the member lists in `DATA-MODEL.md`.

## Acceptance criteria

- AC1 (FR-1..8): Given a valid config tree, When the backend starts, Then it validates config, opens db/index, seeds permissions + user zero roles, rebuilds the search index, cleans `.tmp`, and listens on `listen_addr`. | Given an invalid config (empty path, difficulty 0 or > 10000, ttl 0, page_size > max_search_page_size, text_field_bytes > pdf_size), When boot runs, Then it fails fast, appends to `startup-errors.log`, and exits 1.
- AC2 (FR-9,10,16): Given no session, When I GET `/challenge/read`, Then I receive `{id, difficulty=8192}` and the id is single-use. | Given a stale/already-used challenge or wrong difficulty or malformed solution, When I submit a PoW, Then I get 400 "challenge not issued, expired, or already used"/"PoW verification failed".
- AC3 (FR-11): Given a disallowed domain or an unparseable/over-long email, When I POST `/email/read` (no session), Then 400 "email domain not allowed". | Given a valid email + valid PoW, Then an email is sent (subject+token) and the token is cached.
- AC4 (FR-12): Given the emailed token and a fresh PoW, When I POST `/user/create`, Then I get `{session_token}` and a user (or existing user) now holds `member`. | Given a bad token or expired token, Then 400 "invalid or expired token" and nothing is written.
- AC5 (FR-13,14): Given a stored session token, When I GET `/session/read?id=true&name=true`, Then I get `{id, name}`. | Given a garbage/expired token, Then 401 "invalid session".
- AC6 (FR-15): Given a session and fresh PoW, When I POST `/session/delete`, Then 200 {} "deleted" and the token no longer validates. | Given a missing PoW, Then 400 and the session survives.
- AC7 (FR-17): Given my own user id, When I GET `/user/{id}/read`, Then I see my name (and email_hash only on request). | Given another user's id without `User::Read`, Then 403 "you are denied".
- AC8 (FR-18): Given a valid new name + PoW, When I POST `/user/{id}/update`, Then my name updates (400 "name already taken" on collision; 400 on invalid name). | Given a stale PoW, Then 400 and no change.
- AC9 (FR-19,20): Given an admin session, When I list/rename users, Then results are ordered id-desc and paginated; rename applies validation + uniqueness. | Given a non-admin, Then 403.
- AC10 (FR-21,22): Given my old+new email with two valid PoWs, When I POST `/email/read`, Then two emails are sent and subjects returned; after pasting both tokens with a PoW on `{old_token}\n{new_token}`, the email hash changes, all old sessions die, and a new `{session_token}` is returned. | Given old==new, wrong old email, taken new email, or token/payload mismatch, Then 400 at the respective step.
- AC11 (FR-23,24): Given my account email + PoW + session on `/email/read`, Then a confirmation email is sent; confirming with its token on mode `transfer` re-points all my articles/comments to the least-loaded recycler, deletes my user node and tokens, and returns "deleted". | Given a token bound to another account or an invalid token, Then 400 and nothing transfers.
- AC12 (FR-25,26): Given admin + `hard`, When I delete a user, Then their articles/versions/comment trees and the user node are removed and orphan PDFs deleted. | Given mode `clear` or none, Then 400 "missing or unsupported delete mode (expected \"transfer\" or \"hard\")".
- AC13 (FR-27): Given a valid session, `Article::Create`, and a valid multipart body with a well-formed PDF, When I POST `/article/create`, Then 201 `{article_id, version_id}` and the PDF lands at the hash-derived path. | Given a missing/empty title, an invalid tag list (no `#`, > 8, > 32 chars), an empty note, a duplicate title or duplicate content hash, an oversized/undersized/non-PDF/truncated file, or a missing session/PoW-less request, Then the matching 400/401/403 with nothing written and no file kept.
- AC14 (FR-28): Given article authorship/permission, When I update title/summary/tags, Then fields and tag edges change, orphan tags are cleaned, and 200 `{article_id}`. | Given a conflicting title, Then 400 "title already exists"; given no permission, Then 403.
- AC15 (FR-29): Given `Article::Delete` + mode `transfer`, When I delete an article, Then its owner edge moves to the recycler (404 if the article is missing). | Given mode `hard`, Then cascade delete + PDF cleanup + index rebuild. | Given no recycler exists, Then 500.
- AC16 (FR-30): Given a session, When I GET `/article/read?page=2&limit=8`, Then I get the enriched, id-desc list with pagination flags. | Given `page=0`, Then it clamps to 1; given `limit=9999`, Then it clamps to 200.
- AC17 (FR-31): Given a readable article, When I GET `/article/{id}/read?check_if_is_author=true`, Then I get the detail view plus `is_author`. | Given a private/non-owned article without permission, Then 403; given a missing article, Then 404 "article not found".
- AC18 (FR-32): Given authorship + a strictly greater semver + unique PDF, When I create a version, Then 201 `{version_id}`, latest_version_id updates, and the PDF is stored. | Given an equal/older version, Then 400 "new version must be strictly greater than the latest version"; given a duplicate PDF, Then 400 "identical PDF already exists".
- AC19 (FR-33,34,35): Given a session, Then version list returns newest-first pages; single version read validates the `article_id` cross-check (404 when mismatched); note update validates 1..=1024 ASCII. | Given no permission on the version, Then 403.
- AC20 (FR-36,37,38): Given `Version::Delete` + mode hard, Then the version and its comment trees disappear and latest_version_id refreshes. | Given mode `transfer`, Then 400 "version delete only supports mode \"hard\"". | Given a readable version with no query params, Then the PDF is served as an attachment; given a missing file, Then 404 "PDF file not found".
- AC21 (FR-39,40): Given `Comment::Create` and a valid body, Then a top-level comment on the version is created (201), and replies link to the parent. | Given a missing version/parent or depth ≥ 64, Then 404/400 with no node written.
- AC22 (FR-41,42,43): Given `Comment::Read`, Then comments render as top-level-paged trees with user names; editing re-validates; deleting (transfer) re-points to the recycler or (hard) removes the subtree. | Given no permission, Then 403; given an invalid comment id in the page, Then 500 "invalid comment id" (source behavior — flagged).
- AC23 (FR-44..46): Given `key_word`, ranges, sort, and from/to, When I search, Then hits carry field/label/snippet with `<mark>`, total/total_pages/has_next/has_prev/truncated are correct, and `time` is RFC3339 +08:00. | Given an invalid range/sort/direction or from > to, Then 400; given q > 512 chars, Then 400; given page > total_pages, Then an empty list.
- AC24 (FR-47..51): Given `Role::Manage`, Then roles are CRUD-able with permission/tag/user edge changes. | Given a remove-side change on admin/recycler/member or deleting admin/recycler, Then 400; given mode != hard on delete, Then 400; given a non-`Role::Manage` actor, Then 403.
- AC25 (FR-52,53): Given the assembled principal/resource entities, When Cedar evaluates, Then owner bypass, public-visibility read, role+scope, admin-console, and admin-all policies decide exactly as policy.cedar prescribes; denied actions yield 403 "you are denied"; missing resources yield 404.
- AC26 (FR-54,55): Given exactly one target id, Then `is_author` reflects `Article::Update`/`Comment::Delete` decisions. | Given zero or several ids, Then 400. | Given no recycler holder, Then transfer/delete-account ops fail 500.
- AC27 (FR-56..59): Given a loaded SPA, Then limits load from `/config/read` with defaults until then; every gated call sends `session-token`; a 401 clears the token and flips the gate to "who are you?"; requests abort after 30 s.
- AC28 (FR-60..64): Given navigation, Then each route renders its page under the gates; author-denied pages render "you are denied!"; client validation mirrors server rules; toasts appear on success/error.
- AC29 (FR-65,66): Given a same-origin minted download URL, Then the PDF downloads with the sanitized filename. | Given a foreign-origin URL, Then the request is refused without sending the token. | Given a refresh, Then drafts persist via query params.

## Edge cases & failure modes

- Auth: missing/expired/garbage session token → 401 "invalid session"; challenge consumed before verify → 400; difficulty mismatch → 400; solution/payload over-length caps → 400; email cooldown per recipient (60 s) → 400 "email already sent recently, check your inbox"; SMTP failure → 500 (messages "failed to send ... email"); SMTP wall-clock timeout → 500.
- Email domains: case-insensitive match after trimming leading `@`; empty list means no address is allowed.
- Permissions: any non-allow decision → 403 "you are denied"; assembly missing resource → 404 with per-call message; grant of a permission whose node is missing → 500; unhold of a nonexistent user/role → 500 (internal).
- Concurrency: two identical-PDF uploads race — the second create detects the duplicate hash in the DB txn and returns 400; the losing upload's temp/final file is dropped (PdfUpload RAII removes unkept files); version create and hard-delete race is serialized by the graph write lock; comment depth is checked inside the write transaction; session/cache mutations are atomic via moka entry APIs.
- Upload limits: streamed bytes > `max_pdf_size_bytes` (32 MiB) → 400 "PDF too large: X > Y bytes"; text fields > `max_text_field_bytes` (1 MiB) → 400; multipart body limit = pdf + 5×text + 64 KiB enforced by axum DefaultBodyLimit → 413; non-UTF-8 text → 400.
- Search edge cases: empty q = all; range/sort values comma-joined with unknown token rejected; from>to rejected; huge totals truncate pages at 1024 with `truncated: true`; q over 512 chars rejected; page beyond end returns an empty page with correct has_prev.
- Delete/transfer: no recycler (role missing or no holders) → 500; article with no owner edge → no-op success (transfer_target_ownership returns Ok); deregister when user already gone → sessions purged, 200.
- Partial failures: search index sync/rebuild failures are logged and never fail the mutation (best-effort); PDF file cleanup failures are logged; enrich (author/tags) failures degrade to empty author/tags on the list view; comment user-name lookup failure yields empty names.
- Idempotency: role create is idempotent (returns existing); permission/tag/hold edges are created only if absent; PoW challenges, auth tokens, download tokens, and email-update tokens are single-use; comment/version/user ids are server-generated uuidv7 (no client retry can double-create).
- Time/clock: article/version/comment `created_at` derive from uuidv7 ids (0 for non-v7); search ts derives from the article's latest_version_id.

## Test plan (write these first)

- Unit (common): pow prove/verify round-trip + wrong-difficulty/wrong-length/bad-hex/oversized rejection; email/token/name/tag/text validators (every error variant); tag parsing (multi-`#` segments, dedupe, max count); time helpers (uuidv7 timestamps, min/max for ms); hash determinism.
- Unit (back): `PdfStreamGuard` accept/reject matrix (size, header, version, footer, whitespace tail); content-hash path layout; config validation matrix; PoW gate consume-once; session/auth token cache reverse-index invariants.
- Integration (repo, in-memory agdb): create article/version/comment; duplicate title/hash rejection; version strictly-greater rule; comment depth cap; tag orphan cleanup; ownership transfer to recycler (least-loaded, tie-break); hard delete cascades + latest_version refresh; role/permission/scope edge CRUD; search sync/rebuild counts.
- API (tower oneshot): every route — happy path + each 400/401/403/404/413/500 branch enumerated in the acceptance criteria (auth headers, envelopes, pagination clamps, multipart validation, download mint/consume, role guard rails).
- Authorization: policy.cedar evaluation matrix — owner vs non-owner, public vs private visibility, global vs scoped roles (tag intersection), admin-console resource matching, admin bypass; `authorize_create` synthetic resource.
- E2E (`--features end_to_end`, gated): browser flows — authenticate (challenge→email→token), create/update/delete article, versioning, comments, name/email change, logout, deregister; SMTP sink; PDF download; static hosting.
- Frontend (WASM unit): envelope unwrap, 401 session invalidation, url_encode, download same-origin guard, client validation parity, pagination clamping.

## Improvements & refactors

- [keep-behavior] Extract the 31 route handlers' repeated pagination clamp logic (limit 1..=200, page 1..=10000) into one shared helper (currently copied in five handlers).
- [keep-behavior] Replace the many duplicated `LogicError::internal("database query failed: {e}")` strings with a single `DbError` → `LogicError` conversion.
- [keep-behavior] Fix the frontend delete flows to send `mode` (see notes): `delete_article`/`delete_comment` currently send empty bodies that the backend rejects.
- [keep-behavior] Wire the PDF download consume path end-to-end (see notes) or drop the dead mint/consume branch and serve inline reads only.
- [keep-behavior] Remove dead common types (`CheckEmailRequest/Response`, `DeregisterUserRequest/Confirm`, `VerifySessionRequest`, `AuthorCheckRequest`, typed response structs) or delete the file.
- [keep-behavior] Remove `static/pow-worker.js` or actually use it from `front/src/pow.rs` (the VDF blocks the UI thread today).
- [keep-behavior] Compute `member_count` in `GET /role/read` (currently hardcoded 0).
- [keep-behavior] Guard `member` in role delete like the other REQUIRED_ROLES.
- [keep-behavior] Add an explicit permission check to version-list and article-list reads, or document that lists are session-only by design.
- [keep-behavior] Tests: the unit/integration/E2E matrix above does not exist for most modules; add it.
- [keep-behavior] Reduce startup cost: `rebuild_index` at every boot is O(N); consider a dirty-flag/incremental sync.
- [keep-behavior] Map search range labels to a data-driven localization table (labels are currently hardcoded Chinese while README requires English-only).
- [behavior-change] (opt-in) Move PoW computation into the existing worker to keep the UI responsive.
- [behavior-change] (opt-in) Enforce `visibility` end-to-end (today it is stored but never set or filtered).

## Redesign notes

Map onto `/home/qkun/nail_new/README.md` §4 (skeleton is fixed; no `mod.rs`; ≤16 files per dir; English-only; ≤512 lines/file):
- Backend `interface` ⇐ old `api/` + `api.rs` (route table, session-token extraction, envelope mapping, multipart parsing, PDF serving, `ApiError` = status + envelope). Layer only knows logic + infrastructure.
- Backend `logic` ⇐ old `logic/` (business rules, validation, PoW/email flows, search param parsing, author checks, download mint/consume). No repo access except through `repository`.
- Backend `repository` ⇐ old `repo/` (agdb schema/seed, node/edge CRUD, transfer/hard-delete, tag/token caches, search index).
- Backend `infrastructure` ⇐ old `other/` (conf, app_state, email + SMTP core, pdf stream guard/upload RAII, log writer + prune, server boot/shutdown) plus the current `main.rs` composition.
- Authorization: keep the cedar-policy evaluation (schema.cedar + policy.cedar) as a repository-backed decision module under `repository`/`logic` boundary; entity assembly (principal/resource chains) moves into `repository` with the graph queries.
- `common` ⇐ old `common/src/` unchanged in spirit (pow, hash, name, tag, text, time, request/response DTOs, search params); reorganize into `zzz/yyy/xxx` submodules per the skeleton; delete dead DTOs.
- Frontend `router` ⇐ `router/all.rs` (path → page mapping only).
- Frontend `page` ⇐ `page/**` (layouts, gates, article/version/comment/search pages, notify, pagination, time formatting).
- Frontend `request` ⇐ `req/request/**` (all HTTP, session token, envelope unwrap, timeout, download).
- Frontend `infrastructure` ⇐ `conf.rs` (compile-time config), `limits.rs` (runtime config fetch), `pow.rs` (PoW), storage helpers.
- Interfaces this unit exposes: the 31-route HTTP API (all envelopes) + the compile-time-embedded frontend config; it consumes none internally (self-contained application).

## Definition of done

- [ ] Every FR above (1..66) implemented and covered by at least one test.
- [ ] Every acceptance-criteria scenario passes, including every failure path.
- [ ] All 31 operations respond with the documented envelope shapes and status codes; error branches match the listed messages.
- [ ] Entities/edges written match `DATA-MODEL.md` exactly; write satisfiability holds (no unfilled required column/FK; anonymous ops only touch anonymous-capable data).
- [ ] Every enum value used is a listed member (deletion modes, visibility, ranges, sort fields/directions, permission actions, roles).
- [ ] All edge cases & failure modes above are handled and tested.
- [ ] All `[keep-behavior]` improvements applied; `[behavior-change]` items only with approval.
- [ ] `node scripts/analyze.mjs --check --out <out>` passes with no unresolved callouts.