use agdb::{DbId, DbType, DbTypeMarker};

pub const ENTITY_TYPE_USER: &str = "user";
pub const ENTITY_TYPE_ARTICLE: &str = "article";
pub const ENTITY_TYPE_VERSION: &str = "version";
pub const ENTITY_TYPE_COMMENT: &str = "comment";
pub const ENTITY_TYPE_TAG: &str = "tag";
pub const ENTITY_TYPE_ROLE: &str = "role";
pub const ENTITY_TYPE_PERMISSION: &str = "permission";

pub const EDGE_USER_AUTHOR_ARTICLE: &str = "user_author_article";
pub const EDGE_ARTICLE_HOLD_VERSION: &str = "article_hold_version";
pub const EDGE_USER_AUTHOR_COMMENT: &str = "user_author_comment";
pub const EDGE_COMMENT_ATTACH_VERSION: &str = "comment_attach_version";
pub const EDGE_COMMENT_REPLY_COMMENT: &str = "comment_reply_comment";
pub const EDGE_ARTICLE_APPLY_TAG: &str = "article_apply_tag";
pub const EDGE_USER_HOLD_ROLE: &str = "user_hold_role";
pub const EDGE_ROLE_GRANT_PERMISSION: &str = "role_grant_permission";

pub const KEY_TYPE: &str = "type";
pub const KEY_EMAIL_ADDRESS_HASH: &str = "email_address_hash";
pub const KEY_USER_NAME: &str = "name";
pub const KEY_TITLE: &str = "title";
pub const KEY_SUMMARY: &str = "summary";
pub const KEY_CONTENT_HASH: &str = "content_hash";
pub const KEY_TAG_NAME: &str = "tag_name";
pub const KEY_ROLE_NAME: &str = "role_name";
pub const KEY_PERMISSION_NAME: &str = "permission_name";
pub const KEY_LATEST_VERSION_ID: &str = "latest_version_id";
pub const KEY_VERSION_NOTE: &str = "note";
pub const KEY_COMMENT_CONTENT: &str = "content";
pub const KEY_SOFT_DELETED: &str = "soft_deleted";

pub fn alias_of(kind: &str, business_id: &str) -> String {
    format!("{kind}:{business_id}")
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct UserRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub id: String,
    pub email_address_hash: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct IdRow {
    pub db_id: Option<DbId>,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct RoleRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub id: String,
    pub role_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct PermissionRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub permission_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct ArticleRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub id: String,
    pub title: String,
    pub summary: String,
    pub latest_version_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct VersionRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub id: String,
    pub version_number: String,
    pub content_hash: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct TagRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub id: String,
    pub tag_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct CommentRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub id: String,
    pub content: String,
}
