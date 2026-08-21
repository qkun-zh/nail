/// Taxonomy of stored nodes. One variant per business entity kind; the
/// variant name maps to the `"type"` key value and the alias prefix on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    User,
    Article,
    Version,
    Comment,
    Tag,
    Role,
    Permission,
}

impl NodeKind {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Article => "article",
            Self::Version => "version",
            Self::Comment => "comment",
            Self::Tag => "tag",
            Self::Role => "role",
            Self::Permission => "permission",
        }
    }
}

/// Taxonomy of edges. Variant names map to the `"type"` key value on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    UserAuthorArticle,
    ArticleHoldVersion,
    UserAuthorComment,
    CommentAttachVersion,
    CommentReplyComment,
    ArticleApplyTag,
    UserHoldRole,
    RoleGrantPermission,
}

impl EdgeKind {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::UserAuthorArticle => "user_author_article",
            Self::ArticleHoldVersion => "article_hold_version",
            Self::UserAuthorComment => "user_author_comment",
            Self::CommentAttachVersion => "comment_attach_version",
            Self::CommentReplyComment => "comment_reply_comment",
            Self::ArticleApplyTag => "article_apply_tag",
            Self::UserHoldRole => "user_hold_role",
            Self::RoleGrantPermission => "role_grant_permission",
        }
    }
}

pub(crate) const TYPE_KEY: &str = "type";

/// Node key under which the business id is persisted so row projections
/// can recover it on read.
pub const ID_KEY: &str = "id";

pub(crate) fn alias_of(kind: NodeKind, business_id: &str) -> String {
    format!("{}:{business_id}", kind.key())
}
