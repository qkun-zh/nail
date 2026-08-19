# Exec — Task VIb: unify backend error mapping (BEHAVIOR-PRESERVING)

**Task**: VIb (REFACTOR_PLAN.md §3). **Owner**: P6kRt2.
**Scope root**: `code/back/src/logic/error.rs`,
`code/back/src/logic/{article,authorize,comment,download,email,role,session,tag,user,version}.rs`,
`test/unit/back/logic/error.rs` (additive tests only).

## 1. Requirement

Centralize repository→`LogicError` conversion via `impl From<X> for LogicError`
for every repository error type that reaches `logic/**`, then replace the three
existing mapping styles (named mappers, inline `match` arms, ad-hoc
`LogicError::internal(format!("...: {error}"))` sprinkles) with plain `?`
propagation. Behavior-preserving, EXACTLY:

- The final `LogicError` at every call site is byte-identical (same variant,
  same message string) before and after.
- `database_error` (error.rs) keeps current semantics (all DbError → Internal
  `"database query failed: {error}"`). 404-vs-500 improvement DEFERRED.
- `LogicError` public API unchanged (variants, constructors, `status`,
  `message`, `into_pair`, `database_error` stays pub — a unit test pins it).
- Interface layer untouched (`ApiError::from_logic` already owns conversion).

Acceptance: all back tests per-module green at baseline and after every slice;
fmt clean; clippy 0 warnings; one commit per slice.

## 2. Scope

In: `logic/error.rs`, the 10 logic files above, tests above, my handoff/exec
docs. Out: `interface/**`, `repository/**` (errors stay as-is; only
`logic` converts), `code/front/**`, `code/common/**`, any other agent's paths.
`logic/search.rs` (anyhow), `logic/session.rs` (anyhow `token_key`),
`logic/email.rs` (`SendEmailError` from infrastructure), `logic/authorize.rs`
(cedar errors) are NOT repository-error conversions and stay untouched.

## 3. Design decisions

### Inventory of repository error types reaching logic (complete)

| Error type | Variants | Sites in logic |
|---|---|---|
| `agdb::DbError` | — | ~40 `.map_err(database_error)` sites across 10 files (list below) |
| `CreateArticleError` | AuthorMissing, TitleTaken, ContentHashTaken, Db | article.rs `map_create_article_error` (1 site) |
| `UpdateArticleError` | Missing, TitleTaken, Db | article.rs `map_update_article_error` (1 site) |
| `CreateCommentError` | TargetNotFound, CommentIdExists, CommentTreeTooDeep, Db | comment.rs create_comment (top-level) + create_reply (2 sites, differ on TargetNotFound msg) |
| `CreateVersionError` | ArticleMissing, NotGreater, InvalidNumber, ContentHashTaken, Db | version.rs `map_create_version_error` (1 site) |
| `TransferError` | NoRecycler, Db | user.rs handle_delete_user_transfer (1 site, Db msg `"failed to transfer account assets: {error}"`) |
| `TransferTargetError` | TargetMissing, TargetOwnerMissing, NoRecycler, Db | article.rs + comment.rs transfer delete (2 sites, differ on TargetMissing/TargetOwnerMissing msgs) |
| `UserWriteError` | UserMissing, AlreadyTaken, EmailMismatch, Db | user.rs name (2 sites), email.rs update_user_email (1 site) — 3 distinct mappings |
| `AssemblyError` | ResourceNotFound, Internal(String) | authorize.rs authorize + authorize_anonymous (2 sites, same mapping) |

Plain-DbError sites to convert (`.map_err(database_error)?` → `?`):
article.rs 100,195,213,219,245,251,289; comment.rs 86,96,117,165,200,221,260,
273,279,307,313; version.rs 80,86,153,163,194,225,248,252,258,276,279,308,314,
317; user.rs 66,72,101,112,116,130,138,293,354; email.rs 129,147,222,270;
authorize.rs 25; role.rs 44,64,72,95,106,129,210; tag.rs 26,44,52,74,78,103,
110,131,157; download.rs 27,34; session.rs 61.

DbError sites with DIFFERENT messages (kept per-site, documented — genuine
differences, cannot be a single From):
- comment.rs:140-147 `read_comment_children_page` — `is_not_found` →
  `NotFound("comment not found")`, else `database_error` (per-site DbError rule).
- role.rs:51,172-174,178-181,185-188,191-195,226-227 — `"failed to create
  role: {error}"`, `"failed to grant {permission}: {error}"`,
  `"failed to revoke {permission}: {error}"`, `"failed to hold role for
  {user}: {error}"`, `"failed to unhold role for {user}: {error}"`,
  `"failed to delete role: {error}"` (context-carrying DbError).
- tag.rs:33,117,138,163,188 — `"failed to create tag: {error}"`,
  `"failed to update tag: {error}"`, `"failed to delete tag: {error}"`,
  `"failed to apply tag: {error}"`, `"failed to unapply tag: {error}"`.
- user.rs:79,370,401,420 — `"failed to grant member role: {error}"`,
  `"failed to soft-delete user: {error}"`, `"failed to undelete user: {error}"`,
  `"failed to delete user: {error}"`.

### From impls (all in logic/error.rs, one central place)

Each From maps the FULL variant set of its type, encoding the mapping used by
the majority/sole site; sites whose mapping genuinely differs override the
affected variants locally with `other => other.into()`.

- `From<DbError>` → `database_error(error)`.
- `From<CreateArticleError>` — article.rs `map_create_article_error` body
  verbatim (AuthorMissing→Internal("author not found"), TitleTaken→
  BadRequest("title already exists"), ContentHashTaken→BadRequest("identical
  PDF already exists"), Db→database_error).
- `From<UpdateArticleError>` — `map_update_article_error` body verbatim
  (Missing→NotFound("article not found"), TitleTaken, Db).
- `From<CreateVersionError>` — `map_create_version_error` body verbatim
  (ArticleMissing→NotFound("article not found"), NotGreater→BadRequest("new
  version must be strictly greater than the latest version"), InvalidNumber,
  ContentHashTaken, Db).
- `From<CreateCommentError>` — CommentIdExists→Internal("comment id already
  exists"), CommentTreeTooDeep→BadRequest(format!("comment thread too deep
  (max {MAX_COMMENT_TREE_DEPTH} reply layers)")), Db→database_error, and
  TargetNotFound→NotFound("comment target not found (the version may have been
  removed)") (the top-level message). Top-level site uses plain `?`; the reply
  site overrides only TargetNotFound with its own message. Requires
  `MAX_COMMENT_TREE_DEPTH`, which MOVES from comment.rs to error.rs as
  `pub(crate) const` (same value 64; comment.rs imports it — value and message
  identical).
- `From<TransferError>` — NoRecycler→Internal("no recycler available"),
  Db→Internal(format!("failed to transfer account assets: {error}")) (sole
  site's exact messages).
- `From<TransferTargetError>` — TargetMissing→NotFound("article not found"),
  TargetOwnerMissing→Internal("article has no owner"), NoRecycler→Internal("no
  recycler available"), Db→database_error (article site's mapping; article
  delete-transfer uses plain `?`). Comment delete-transfer overrides
  TargetMissing (`"comment not found"`) and TargetOwnerMissing (`"comment has
  no owner"`) locally.
- `From<UserWriteError>` — `name_update_error` body verbatim (AlreadyTaken→
  BadRequest("name already taken"), UserMissing→Unauthorized("user not found"),
  EmailMismatch→Internal("unexpected email mismatch"), Db→Internal(format!(
  "failed to update name: {error}"))). handle_update_name uses `?`; the admin
  name site overrides only UserMissing→NotFound("user not found");
  update_user_email keeps its full local match (all four variants differ).
- `From<AssemblyError>` — `map_assembly_error` body verbatim (ResourceNotFound→
  NotFound("resource not found"), Internal(msg)→Internal(msg)); both authorize
  sites use `?`.

Rationale for the override pattern: a `From` impl must be exhaustive over the
enum. Where sites genuinely differ, the From encodes one mapping (the most
used/least surprising) and the divergent site(s) override locally — the
"preserve per-site difference" rule from the task. Documented here per type.

### Call-site rewrites

- All plain `.map_err(database_error)?` → `?` (DbError list above).
- article.rs: create → `Err(error.into())` after `drop(upload)`; update → `?`;
  delete-transfer → `?`; delete mappers.
- comment.rs: create_comment → `?`; create_reply →
  `.map_err(|e| match e { TargetNotFound => <reply msg>, other => other.into() })?`;
  delete-transfer →
  `.map_err(|e| match e { TargetMissing => NotFound("comment not found"),
  TargetOwnerMissing => Internal("comment has no owner"), other => other.into() })?`;
  delete `map_create_comment_error`, `map_transfer_error`.
- version.rs: create → `Err(error.into())`; delete `map_create_version_error`.
- user.rs: create_user `Err(error)` → `Err(error.into())` (cache-restore side
  effect kept); handle_update_name → `?`; handle_admin_update_name →
  `.map_err(|e| match e { UserMissing => NotFound("user not found"), other =>
  other.into() })?`; handle_delete_user_transfer → `?`; delete `name_update_error`.
- authorize.rs: both sites → `?`; delete `map_assembly_error`.
- email.rs: update_user_email keeps its full local match (all variants differ
  from From<UserWriteError>).

## 4. Slice breakdown

| # | Red | Green | Exit test |
|---|---|---|---|
| 1 | New From tests in test/unit/back/logic/error.rs fail to compile (no From impls yet) | error.rs: 9 From impls + `MAX_COMMENT_TREE_DEPTH` moved in; comment.rs imports it; From tests pass | fmt, clippy, logic_ + http_ groups (574 still green) |
| 2 | Scratch-disabled `From<DbError>` → `?` sites fail to compile (observed once) | All plain DbError sites converted to `?`; user.rs create_user `error.into()` | fmt, clippy, all 5 module groups |
| 3 | Named mappers deleted first → call sites fail to compile (observed) | Call sites converted to `?`/overrides; mappers removed; `database_error` import cleanup where unused | fmt, clippy, all 5 module groups |

Slices 2 and 3 are behavior-preserving mechanical rewrites; the "red" is the
compile-error that the From impls make possible (each demonstrated once). The
behavior pin is the 574 pre-existing tests plus the slice-1 From tests.

## 5. Open unknowns

- U1 (source): `DbError::query(DbErrorType, impl Into<String>)` constructor —
  agdb-0.13.2 `src/db/db_error.rs:83`; Display includes source location
  (verified, `db_error.rs:132-148`). From tests construct `DbError::query(
  DbErrorType::Query, "boom")` and assert against `format!("database query
  failed: {db}")` — probe is the slice-1 test run itself.
- U2 (grep): no test/interface/infrastructure code references the named
  mappers (`map_*`, `name_update_error`) or `database_error` outside
  `logic/**` + the pin test — confirmed by search. Deleting them is safe.
- U3 (source): no other `From` impls for `LogicError` exist (error.rs has
  none) and repository types are local to this crate — the new impls are
  unambiguous; `?` conversion needs no imports.

## 6. Verification plan

- Correctness: slice-1 From tests pin every variant→LogicError mapping
  (variant + exact message); 574 pre-existing tests are the behavior pin.
- Behavior change: none by construction; every message string is carried
  verbatim into the From/override arms; http tests (139) pin status+message
  at the wire.
- Time/space: identical (conversion is a match, same allocations).
- Performance: no delta.

## 7. Risks

- Missing a DbError site → clippy `unused_must_use`/behavior drift; mitigated
  by the complete inventory above + grep re-check per slice.
- `From` with wrong message → caught by slice-1 tests + existing message-pinning
  tests ("title already exists", "identical PDF already exists", "name already
  taken", "comment thread too deep (max 64 reply layers)").
- Concurrent agent touching shared docs → re-read before each gate; stage only
  my own paths.
- OOM: check `uptime` before every build; run tests split per module.
- Rollback: one commit per slice; revert restores prior state.

## 8. Constraints

No status/message/variant changes. No `unwrap`/`expect`/new panics. No comments
restating code. English only. Never hand-edit Cargo.lock; never touch
target/dist/data/log. No `sed`/`awk` for editing — Read/Edit/Write only. One
commit per slice, clean tree. Stage only my own paths; never `git add -A`/
`git add .`. Do not touch `code/front/**` or `code/common/**`.

## 9. Questions

- Q1: `From<CreateCommentError>` needs `MAX_COMMENT_TREE_DEPTH` for its
  `CommentTreeTooDeep` message. I plan to move that const from comment.rs to
  error.rs (`pub(crate)`, same value) so the central From stays in error.rs.
  Acceptable?
- Q2: `From<TransferTargetError>` encodes the ARTICLE messages; the comment
  site overrides two variants. This keeps both sites byte-identical while
  using `?` maximally. Acceptable (vs. keeping both sites as full local
  matches and a partial From with an unreachable arm — rejected, no panics)?
- Q3: `From<UserWriteError>` encodes the name mapping; the email site keeps its
  full local match because all four of its variants differ. Acceptable?

## Change log

- 2026-08-19: Adoption gate APPROVED by orchestrator. Q1, Q2, Q3 all
  APPROVED as designed. Baseline measured: 574/574 (config 11, infra 45, logic
  272, repo 107, http 139; plan doc's 576 was stale).
- 2026-08-19: Slice 1 (From impls + const move + From tests) committed
  `985f89b` — gate: fmt clean, clippy 0 (dep note only), logic 281, http 139,
  repo 107, infra 45, config 11.
- 2026-08-19: Slice 2 (plain DbError sites → `?`) committed `657509d` — red
  demonstrated once (scratch-disabled `From<DbError>`, 67 E0277), gate: fmt
  clean, clippy 0, all 5 module groups green (583).
- 2026-08-19: Slice 3 (named mappers deleted, call sites converted) committed
  `72956ab` — red demonstrated once (7 E0425), gate: fmt clean, clippy 0, all
  5 module groups green (583). `database_error` remains pub in error.rs (pin
  test) and in comment.rs only for the read_comment_children_page special
  site; email.rs keeps its full local UserWriteError match (Q3).