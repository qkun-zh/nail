use database::{ID_KEY, NodeKind, Row, Value};

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

/// Indexes ensured at open time; uniqueness lookups rely on them.
pub const INDEX_KEYS: &[&str] = &[
    KEY_EMAIL_ADDRESS_HASH,
    KEY_USER_NAME,
    KEY_TITLE,
    KEY_CONTENT_HASH,
    KEY_TAG_NAME,
    KEY_ROLE_NAME,
    KEY_PERMISSION_NAME,
];

fn text(key: &str, value: &str) -> (String, Value) {
    (key.to_string(), Value::Text(value.to_string()))
}

/// Business-id-only projection readable from any kind. Read-only: never
/// insert it, so [`Row::KIND`] is never exercised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdRow {
    pub id: String,
}

impl Row for IdRow {
    const KIND: NodeKind = NodeKind::User;

    fn business_id(&self) -> &str {
        &self.id
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        Vec::new()
    }

    fn from_lookup(lookup: &dyn database::ValueLookup) -> Result<Self, database::Error> {
        Ok(Self {
            id: lookup.required_text(ID_KEY)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRow {
    pub id: String,
    pub email_address_hash: String,
    pub name: String,
}

impl Row for UserRow {
    const KIND: NodeKind = NodeKind::User;

    fn business_id(&self) -> &str {
        &self.id
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        vec![
            text(KEY_EMAIL_ADDRESS_HASH, &self.email_address_hash),
            text(KEY_USER_NAME, &self.name),
        ]
    }

    fn from_lookup(lookup: &dyn database::ValueLookup) -> Result<Self, database::Error> {
        Ok(Self {
            id: lookup.required_text(ID_KEY)?,
            email_address_hash: lookup.required_text(KEY_EMAIL_ADDRESS_HASH)?,
            name: lookup.required_text(KEY_USER_NAME)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleRow {
    pub id: String,
    pub role_name: String,
}

impl Row for RoleRow {
    const KIND: NodeKind = NodeKind::Role;

    fn business_id(&self) -> &str {
        &self.id
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        vec![text(KEY_ROLE_NAME, &self.role_name)]
    }

    fn from_lookup(lookup: &dyn database::ValueLookup) -> Result<Self, database::Error> {
        Ok(Self {
            id: lookup.required_text(ID_KEY)?,
            role_name: lookup.required_text(KEY_ROLE_NAME)?,
        })
    }
}

/// Permission nodes use the permission name as their business id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRow {
    pub permission_name: String,
}

impl Row for PermissionRow {
    const KIND: NodeKind = NodeKind::Permission;

    fn business_id(&self) -> &str {
        &self.permission_name
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        vec![text(KEY_PERMISSION_NAME, &self.permission_name)]
    }

    fn from_lookup(lookup: &dyn database::ValueLookup) -> Result<Self, database::Error> {
        Ok(Self {
            permission_name: lookup.required_text(KEY_PERMISSION_NAME)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleRow {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub latest_version_id: Option<String>,
}

impl Row for ArticleRow {
    const KIND: NodeKind = NodeKind::Article;

    fn business_id(&self) -> &str {
        &self.id
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        let mut row = vec![
            text(KEY_TITLE, &self.title),
            text(KEY_SUMMARY, &self.summary),
        ];
        if let Some(latest) = &self.latest_version_id {
            row.push(text(KEY_LATEST_VERSION_ID, latest));
        }
        row
    }

    fn from_lookup(lookup: &dyn database::ValueLookup) -> Result<Self, database::Error> {
        Ok(Self {
            id: lookup.required_text(ID_KEY)?,
            title: lookup.required_text(KEY_TITLE)?,
            summary: lookup.required_text(KEY_SUMMARY)?,
            latest_version_id: lookup.optional_text(KEY_LATEST_VERSION_ID)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRow {
    pub id: String,
    pub version_number: String,
    pub content_hash: String,
    pub note: String,
}

impl Row for VersionRow {
    const KIND: NodeKind = NodeKind::Version;

    fn business_id(&self) -> &str {
        &self.id
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        vec![
            text(KEY_CONTENT_HASH, &self.content_hash),
            text(KEY_VERSION_NOTE, &self.note),
            (
                "version_number".to_string(),
                Value::Text(self.version_number.clone()),
            ),
        ]
    }

    fn from_lookup(lookup: &dyn database::ValueLookup) -> Result<Self, database::Error> {
        Ok(Self {
            id: lookup.required_text(ID_KEY)?,
            version_number: lookup.required_text("version_number")?,
            content_hash: lookup.required_text(KEY_CONTENT_HASH)?,
            note: lookup.required_text(KEY_VERSION_NOTE)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRow {
    pub id: String,
    pub tag_name: String,
}

impl Row for TagRow {
    const KIND: NodeKind = NodeKind::Tag;

    fn business_id(&self) -> &str {
        &self.id
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        vec![text(KEY_TAG_NAME, &self.tag_name)]
    }

    fn from_lookup(lookup: &dyn database::ValueLookup) -> Result<Self, database::Error> {
        Ok(Self {
            id: lookup.required_text(ID_KEY)?,
            tag_name: lookup.required_text(KEY_TAG_NAME)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentRow {
    pub id: String,
    pub content: String,
}

impl Row for CommentRow {
    const KIND: NodeKind = NodeKind::Comment;

    fn business_id(&self) -> &str {
        &self.id
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        vec![text(KEY_COMMENT_CONTENT, &self.content)]
    }

    fn from_lookup(lookup: &dyn database::ValueLookup) -> Result<Self, database::Error> {
        Ok(Self {
            id: lookup.required_text(ID_KEY)?,
            content: lookup.required_text(KEY_COMMENT_CONTENT)?,
        })
    }
}
