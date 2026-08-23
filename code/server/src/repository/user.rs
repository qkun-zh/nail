use std::collections::HashMap;

use database::{Database, Error, NodeKind};

use crate::repository::access::GraphRead;
use crate::repository::schema::{KEY_EMAIL_ADDRESS_HASH, KEY_USER_NAME, UserRow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserEntry {
    pub email_address_hash: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserListItem {
    pub id: String,
    pub name: String,
}

#[derive(Debug)]
pub enum UserWriteError {
    UserMissing,
    AlreadyTaken,
    EmailMismatch,
    Db(Error),
}

impl From<Error> for UserWriteError {
    fn from(error: Error) -> Self {
        Self::Db(error)
    }
}

impl std::fmt::Display for UserWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserMissing => formatter.write_str("user not found"),
            Self::AlreadyTaken => formatter.write_str("value already taken"),
            Self::EmailMismatch => formatter.write_str("email hash does not match"),
            Self::Db(error) => write!(formatter, "database query failed: {error}"),
        }
    }
}

impl std::error::Error for UserWriteError {}

pub fn create_user(db: &Database, email_address_hash: &str) -> Result<String, Error> {
    db.write(|scope| {
        if let Some(existing) = scope.find_by_key(KEY_EMAIL_ADDRESS_HASH, email_address_hash)?
            && let Some(row) = scope.scope_read_node::<UserRow>(existing)?
        {
            return Ok(row.id);
        }
        let user_id = uuid::Uuid::now_v7().to_string();
        let row = UserRow {
            id: user_id.clone(),
            email_address_hash: email_address_hash.to_string(),
            name: "anonymous".to_string(),
        };
        scope.insert_node(&row)?;
        Ok(user_id)
    })
}

pub fn read_user_by_email_address_hash(
    db: &Database,
    email_address_hash: &str,
) -> Result<Option<String>, Error> {
    db.read(|scope| {
        Ok(scope
            .find_by_key(KEY_EMAIL_ADDRESS_HASH, email_address_hash)?
            .and_then(|id| scope.scope_read_node::<UserRow>(id).transpose())
            .transpose()?
            .map(|row| row.id))
    })
}

pub fn read_user(db: &Database, user_id: &str) -> Result<Option<UserEntry>, Error> {
    db.read(|scope| {
        let Some(id) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(None);
        };
        let row = scope.scope_read_node::<UserRow>(id)?.ok_or_else(|| {
            Error::Invalid(format!("user {user_id} exists but has no readable row"))
        })?;
        Ok(Some(UserEntry {
            email_address_hash: row.email_address_hash,
            name: row.name,
        }))
    })
}

pub fn read_user_names(
    db: &Database,
    user_ids: &[String],
) -> Result<HashMap<String, String>, Error> {
    db.read(|scope| {
        let mut names = HashMap::new();
        for user_id in user_ids {
            let Some(node) = scope.resolve(NodeKind::User, user_id)? else {
                continue;
            };
            if let Some(row) = scope.scope_read_node::<UserRow>(node)? {
                names.insert(user_id.clone(), row.name);
            }
        }
        Ok(names)
    })
}

pub fn update_user_name(db: &Database, user_id: &str, name: &str) -> Result<(), UserWriteError> {
    db.write(|scope| {
        let Some(id) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(Err(UserWriteError::UserMissing));
        };
        let taken = scope
            .find_by_key(KEY_USER_NAME, name)?
            .is_some_and(|other| other != id);
        if taken {
            return Ok(Err(UserWriteError::AlreadyTaken));
        }
        scope.set_key(id, KEY_USER_NAME, name.to_string())?;
        Ok(Ok(()))
    })
    .map_err(UserWriteError::from)
    .and_then(std::convert::identity)
}

pub fn update_user_email(
    db: &Database,
    user_id: &str,
    old_email_address_hash: &str,
    new_email_address_hash: &str,
) -> Result<(), UserWriteError> {
    db.write(|scope| {
        let Some(id) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(Err(UserWriteError::UserMissing));
        };
        let current_hash = scope
            .scope_read_node::<UserRow>(id)?
            .map(|row| row.email_address_hash)
            .unwrap_or_default();
        if current_hash != old_email_address_hash {
            return Ok(Err(UserWriteError::EmailMismatch));
        }
        let taken = scope
            .find_by_key(KEY_EMAIL_ADDRESS_HASH, new_email_address_hash)?
            .is_some_and(|other| other != id);
        if taken {
            return Ok(Err(UserWriteError::AlreadyTaken));
        }
        scope.set_key(
            id,
            KEY_EMAIL_ADDRESS_HASH,
            new_email_address_hash.to_string(),
        )?;
        Ok(Ok(()))
    })
    .map_err(UserWriteError::from)
    .and_then(std::convert::identity)
}

pub fn read_users(db: &Database) -> Result<Vec<UserListItem>, Error> {
    db.read(|scope| {
        let nodes = scope.all_nodes(NodeKind::User)?;
        let rows = scope.scope_read_nodes::<UserRow>(&nodes)?;
        let mut users = Vec::with_capacity(rows.len());
        for (node, row) in nodes.into_iter().zip(rows) {
            if crate::repository::delete::has_soft_deleted_flag(scope, node)? {
                continue;
            }
            users.push(UserListItem {
                id: row.id,
                name: row.name,
            });
        }
        users.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(users)
    })
}
