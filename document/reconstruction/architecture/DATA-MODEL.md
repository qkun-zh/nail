# Data model

| Setting | Value |
| --- | --- |
| Mode | `redesign` |
| Level | `complex` |
| Fidelity | `describe` |
| TDD | `on` (build test-first) |
| Generated with | `reconstruct@2.17.0` |

Ground truth: `data/schema/code/back/src/repo/schema.rs` (agdb graph),
`data/schema/code/back/src/authorization/schema.cedar` (Cedar), and the
`common` structs in the reference tree. IDs are UUIDv7; hashing is ascon-xof128.

## Graph entities (agdb)

### user
- `id`: string uuidv7 (PK, alias `user:{id}`)
- `email_address_hash`: string, 32 lowercase hex (NOT NULL, indexed `KEY_EMAIL_ADDRESS_HASH`; ascon-xof128(email))
- `name`: string (NOT NULL, indexed `KEY_USER_NAME`; default = id minus dashes; validated 1..=32 ascii alnum/-/_)
- relations: N→article (owner, `user_to_article`); N→comment (owner, `user_to_comment`); N→role (`user_hold_role`)
- uniques: email_address_hash, name (enforced by index lookup in write transactions)

### article
- `id`: string uuidv7 (PK, alias `article:{id}`)
- `title`: string (NOT NULL, indexed `KEY_TITLE`, ASCII 1..=200)
- `summary`: string (NOT NULL, ASCII 1..=2000, newlines allowed)
- `visibility`: string, default `'public'`; values public|private (never set or filtered by any endpoint — inert, see notes)
- `latest_version_id`: string uuidv7, nullable (id of the highest-sorting version)
- relations: owner user→ (`user_to_article`); N→version (`article_to_version`); N→tag (`article_to_tag`)
- uniques: title; content hash of latest version (checked at write)

### version
- `id`: string uuidv7 (PK, alias `version:{id}`)
- `version_number`: string semver (NOT NULL; strictly greater than existing max on create)
- `content_hash`: string, 32 lowercase hex (NOT NULL, indexed `KEY_CONTENT_HASH`; ascon-xof128 of PDF bytes)
- `note`: string (NOT NULL, ASCII 1..=1024, newlines allowed)
- relations: parent article→ (`article_to_version`); N→comment (`comment_to_version`)
- uniques: content_hash

### comment
- `id`: string uuidv7 (PK, alias `comment:{id}`)
- `content`: string (NOT NULL, ASCII 1..=1024, newlines allowed)
- relations: owner user→ (`user_to_comment`); parent version→ (`comment_to_version`, top-level); parent comment→ (`comment_to_comment`, reply); tree depth ≤ 64

### tag
- `id`: string uuidv7 (PK, alias `tag:{id}`)
- `tag_name`: string (NOT NULL, indexed `KEY_TAG_NAME`; `'#xxx'`, 2..=32 chars, ascii alnum/-/_ after `#`)
- relations: N←article (`article_to_tag`); N←role (`role_apply_tag`, scope)
- uniques: tag_name

### role
- `role_name`: string (NOT NULL, indexed `KEY_ROLE_NAME`; 1..=64 ascii alnum/-/_; REQUIRED_ROLES = admin | recycler | member)
- relations: N←user (`user_hold_role`); N→permission (`role_grant_permission`); N→tag (`role_apply_tag`)
- uniques: role_name

### permission
- `permission_name`: string (NOT NULL, indexed `KEY_PERMISSION_NAME`; seeded from schema.cedar actions, 16 total)
- relations: N←role (`role_grant_permission`)
- uniques: permission_name

## Cedar entities (authorization schema)

- **User**: `global_role: Bool` (true if any held role has empty scopes); `scopes: Set<Tag>` (union of held roles' tags); parents = held Role entities
- **Role**: parents = granted Action entities
- **Tag**: used as scope members
- **Visibility**: values public | private (from repo types)
- **Article**: `owner: User` (from `user_to_article` edge), `visibility: Visibility` (default private when missing), `required_scopes: Set<Tag>` (article's tags)
- **Version**: `owner`, `visibility`, `required_scopes`; parent = Article entity (chain)
- **Comment**: `owner` (comment author, may differ from article owner), `visibility`, `required_scopes`; parent = Version entity (chain)
- **System**: resource `System::"admin-console"` for `User::*` / `Role::Manage` actions

## Shared DTOs (common crate)

### Challenge (common::pow)
- `id`: Uuid v7 (single-use, TTL 300s); `difficulty`: u64 (= 8192; verify() rejects mismatch)

### Pow (common::pow)
- `challenge`: Challenge (must be server-issued); `solution`: String hex of 96 bytes (192 hex chars, ≤4096 chars; VDF output 48 + proof 48); `payload`: String ≤4096 bytes (semantics per endpoint)

### ProveInput (common::pow)
- `challenge`, `payload` (≤4096 bytes)

### ResponseEnvelope\<T\> (common::response)
- `code`: u16 (mirrors HTTP status); `data`: Option\<T\> (null on errors); `message`: String ("ok"/"created"/"deleted" on success; reason on error; "internal server error" for Internal)

### Request payloads (common::request)
- `EmailReadRequest`: pow? / old_email_pow? / new_email_pow? (both-or-neither for the dual pair)
- `TokenRequest` / `LogoutRequest` / `NameSetRequest` / `CheckEmailRequest` / `DeregisterUserRequest` / `DeregisterUserConfirmRequest`: `pow: Pow` (payload semantics differ per endpoint)
- `UserUpdateRequest`: pow? (self-rename/email confirm), name? (admin rename), old_email_token? + new_email_token? (both-or-neither)
- `UserDeleteRequest`: mode? (transfer|hard), pow (transfer branch)
- `DeleteBody`: mode? (transfer|hard; version/role: hard only)
- `UpdateArticleRequest`: title (1..=200 ASCII), summary (1..=2000 ASCII), tags (default ""; 1..=8 hashtags)
- `CreateCommentRequest`: content (1..=1024 ASCII)
- `CreateRoleRequest`: name (1..=64 ascii alnum/-/_); `RoleUpdateRequest`: permissions?/tags?/users? each Option\<ChangeList\> (add/remove Vec\<String\>)

### Search (common::search + logic::article_search)
- `ArticleSearchParams`: q? (≤512 chars), ranges? (comma list title|summary|author|comment|note|tag), sort? (comma list field:direction), from?/to? (epoch secs, from ≤ to), limit? (1..=200), page? (1..=1024 search / 1..=10000 list)
- `SearchHit` (shared shape): field, label (Chinese label 标题/摘要/作者/评论/版本说明/标签 — hardcoded, see notes), snippet (may contain `<mark>...</mark>`)
- `SearchArticleItem` (shared shape): id, title, author, time (RFC3339 +08:00), hits (only for requested ranges)
- `SearchPage`: article_list, total, page, total_pages (capped at max_search_pages), has_more, has_prev, truncated

### RoleListItem (logic::role)
- name, permissions, scopes, member_count (always 0 in source, see notes)

## Cache entries (moka)

- `AuthenticateTokenEntry`: email_address_hash, email_subject; keyed by hash(token); reverse index authenticate_by_email_hash; TTL 8000s
- `SessionTokenEntry`: user_id; keyed by hash(token); reverse index session_by_user; TTL 8000s
- `EmailUpdateTokenEntry`: old_email_address_hash, new_email_address_hash, token_from_old_email_hash, token_from_new_email_hash; keyed by user_id; one per user (replaced on re-request)
- `DeregisterTokenEntry`: user_id, email_address_hash; keyed by hash(token); reverse index deregister_by_user; TTL 8000s
- `DownloadTokenEntry`: version_id, user_id; keyed by hash(token); TTL 60s, single-use

## Enums / domain values

| Enum | Members |
| --- | --- |
| DeleteMode (request bodies) | transfer \| hard (version/role delete: hard only; user delete: transfer\|hard) |
| Visibility | public \| private |
| SearchRange | title \| summary \| author \| comment \| note \| tag |
| SearchSortField | time \| title \| author |
| SearchSortDirection | asc \| desc |
| PermissionAction (schema.cedar, 16) | Article::Create, Article::Read, Article::Update, Article::Delete, Version::Create, Version::Read, Version::Update, Version::Delete, Comment::Create, Comment::Read, Comment::Update, Comment::Delete, User::Read, User::Update, User::Delete, Role::Manage |
| RequiredRole | admin \| recycler \| member |
| LogicError | BadRequest \| Unauthorized \| Forbidden \| NotFound \| Internal |
| PdfStreamGuardError | TooLarge{size,max} \| TooSmall{size} \| BadHeader \| BadVersion \| BadFooter |
| SendEmailError | RateLimited \| Smtp(anyhow::Error) |
| NotificationType (frontend) | Info \| Success \| Error |
| CommentLevel (frontend url parsing) | VersionPage \| VersionComments \| Comment(String) \| DeleteComment(String) \| Invalid |
| PdfUploadPhase | Received{tmp} \| Placed{final_path} \| Kept |
| NameError (common::name) | Empty \| TooLong \| ContainsForbiddenChar(char) |
| TagNameError (common::tag) | Empty \| MissingHash \| TooLong \| ContainsForbiddenChar(char) |
| TagNamesError (common::tag) | Name(TagNameError) \| TooManyTags{max_count} |
| TextError (common::text) | Empty \| TooLong{max_chars} \| ContainsForbiddenChar(char) |
| CreateArticleError | AuthorNotFound \| TitleAlreadyExists \| TagNotFound (dead, never produced) \| Db(DbError) |
| UpdateArticleError | NotFound \| TitleAlreadyExists \| TagNotFound \| Db(DbError) |
| CreateVersionError | ArticleNotFound \| VersionNotGreater \| InvalidVersion \| ContentHashExists \| Db(DbError) |
| CreateCommentError | TargetNotFound \| CommentIdExists \| CommentTreeTooDeep \| Db(DbError) |
| TargetTransferError | TargetNotFound \| NoRecycler \| Db(DbError) |
| UserWriteError | UserMissing \| AlreadyTaken \| Db(DbError) |
| Cedar Resource (entity_store) | Article(String) \| Version(String) \| Comment(String) \| System(String) (System variant dead code — only literal admin-console used) |
