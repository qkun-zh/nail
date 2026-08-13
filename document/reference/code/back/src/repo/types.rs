
use agdb::{DbId, DbType, DbTypeMarker};

pub const ENTITY_TYPE_USER: &str = "user";
pub const ENTITY_TYPE_ARTICLE: &str = "article";
pub const ENTITY_TYPE_VERSION: &str = "version";
pub const ENTITY_TYPE_COMMENT: &str = "comment";
pub const ENTITY_TYPE_TAG: &str = "tag";
pub const ENTITY_TYPE_ROLE: &str = "role";
pub const ENTITY_TYPE_PERMISSION: &str = "permission";

pub const EDGE_USER_TO_ARTICLE: &str = "user_to_article";
pub const EDGE_ARTICLE_TO_VERSION: &str = "article_to_version";
pub const EDGE_USER_TO_COMMENT: &str = "user_to_comment";
pub const EDGE_COMMENT_TO_VERSION: &str = "comment_to_version";
pub const EDGE_COMMENT_TO_COMMENT: &str = "comment_to_comment";
pub const EDGE_ARTICLE_TO_TAG: &str = "article_to_tag";
pub const EDGE_USER_HOLD_ROLE: &str = "user_hold_role";
pub const EDGE_ROLE_GRANT_PERMISSION: &str = "role_grant_permission";
pub const EDGE_ROLE_APPLY_TAG: &str = "role_apply_tag";

pub const KEY_TYPE: &str = "type";
pub const KEY_ID: &str = "id";
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

pub const VISIBILITY_PUBLIC: &str = "public";
pub const VISIBILITY_PRIVATE: &str = "private";

pub fn alias_of(kind: &str, business_id: &str) -> String {
    format!("{kind}:{business_id}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserEntry {
    pub email_address_hash: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionEntry {
    pub version_number: String,
    pub content_hash: String,
    pub note: String,
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
pub struct AuthorRow {
    pub db_id: Option<DbId>,
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct ArticleRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub id: String,
    pub title: String,
    pub summary: String,
    pub visibility: Option<String>,
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
pub struct CommentRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub id: String,
    pub content: String,
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
pub struct IdRow {
    pub db_id: Option<DbId>,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct RoleRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub role_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, DbType, DbTypeMarker)]
pub struct PermissionRow {
    pub db_id: Option<DbId>,
    #[agdb(rename = "type")]
    pub entity_type: String,
    pub permission_name: String,
}


#[derive(Debug, Clone)]
pub struct AuthenticateTokenEntry {
    pub email_address_hash: String,
    pub email_subject: String,
}

#[derive(Debug, Clone)]
pub struct SessionTokenEntry {
    pub user_id: String,
}

#[derive(Debug, Clone)]
pub struct EmailUpdateTokenEntry {
    pub old_email_address_hash: String,
    pub new_email_address_hash: String,
    pub token_from_old_email_hash: String,
    pub token_from_new_email_hash: String,
}

#[derive(Debug, Clone)]
pub struct DeregisterTokenEntry {
    pub user_id: String,
    pub email_address_hash: String,
}

#[derive(Debug, Clone)]
pub struct DownloadTokenEntry {
    pub version_id: String,
    pub user_id: String,
}
