## Task VIb: Unify backend error mapping (behavior-preserving)

**Owner**: P6kRt2
**Exec doc**: `document/exec/M4pp_error-mapping-unify.md`
**Status**: COMPLETE — all 3 slices committed, handoff pending orchestrator final gate review

### A. Centralize repository error mapping via From impls (slice 1)

1. **Status**: COMMITTED `985f89b` — gate green: fmt, clippy 0, all 5 module groups.
   - `code/back/src/logic/error.rs`: 9 `From<RepoError> for LogicError` impls
     (DbError, CreateArticleError, UpdateArticleError, CreateCommentError,
     CreateVersionError, TransferError, TransferTargetError, UserWriteError,
     AssemblyError) + `pub(crate) const MAX_COMMENT_TREE_DEPTH` moved from
     comment.rs.
   - `code/back/src/logic/comment.rs`: imports the moved const.
   - `test/unit/back/logic/error.rs`: 9 new From-conversion tests pinning
     variant + exact message (red: tests failed to compile before the impls).
   - Decisions (user approved): Q1 const move; Q2 `From<TransferTargetError>`
     encodes ARTICLE messages (comment site overrides 2 variants); Q3
     `From<UserWriteError>` encodes name mapping (email site keeps full local
     match).

### B. Convert plain DbError sites to `?` (slice 2)

2. **Status**: COMMITTED `657509d` — red demonstrated once (scratch-disabled
   `From<DbError>` → 67 E0277), restored; gate green (583).
   - All plain `.map_err(database_error)?` → `?` in article.rs, comment.rs,
     version.rs, user.rs, email.rs, authorize.rs, role.rs, tag.rs, download.rs,
     session.rs; user.rs create_user → `Err(error.into())` (cache-restore kept).
   - Kept per-site differences: comment.rs read_comment_children_page
     `is_not_found` special case; custom-message DbError wraps in role.rs (6),
     tag.rs (5), user.rs (4: grant member role / soft-delete / undelete /
     delete); `database_error` import dropped where unused.

### C. Delete named mappers, convert remaining sites (slice 3)

3. **Status**: COMMITTED `72956ab` — red demonstrated once (7 E0425 after
   deleting mappers), gate green (583).
   - Deleted: map_create_article_error, map_update_article_error,
     map_transfer_error, map_create_comment_error, map_create_version_error,
     name_update_error, map_assembly_error.
   - Converted: article.rs create `Err(error.into())` + update `?` +
     delete-transfer `?`; comment.rs create_comment `?`, create_reply
     TargetNotFound override ("reply target not found (the parent comment may
     have been removed)"), delete-transfer TargetMissing/TargetOwnerMissing
     override ("comment not found" / "comment has no owner"); version.rs create
     `Err(error.into())`; user.rs handle_update_name `?`, admin name site
     UserMissing→NotFound override, delete-transfer `?`; authorize.rs both
     sites `?`.
   - `database_error` still pub (pin test) and used only in error.rs From impls
     + comment.rs special site. email.rs local match untouched.

### Notes for the user

- No behavior change: every message/variant is byte-identical; wire behavior
  pinned by 139 http tests plus the 9 From tests; full suite 583 (config 11,
  infra 45, logic 281, repo 107, http 139) green at every slice.
- Interface layer untouched (`ApiError::from_logic` path unchanged).
- Untracked files not staged: two rustc-ICE dumps in `code/back/`
  (`rustc-ice-2026-08-19T13_42_*.txt`, pre-existing crash artifacts — safe to
  delete, not mine to commit).
