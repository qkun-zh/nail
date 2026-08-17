use std::collections::HashMap;

use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{
    DbHandle, find_by_index_sync, read_rows_sync, resolve_node_id_sync,
};
use crate::repository::schema::{
    ENTITY_TYPE_USER, IdRow, KEY_EMAIL_ADDRESS_HASH, KEY_TYPE, KEY_USER_NAME, UserRow, alias_of,
};

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
    Db(DbError),
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

pub async fn create_user(db: &DbHandle, email_address_hash: &str) -> Result<String, DbError> {
    let mut guard = db.write().await;
    let ids = find_by_index_sync(&guard, KEY_EMAIL_ADDRESS_HASH, email_address_hash)?;
    if let Some(user_id) = ids.first()
        && let Some(row) = read_rows_sync::<IdRow>(&guard, &[*user_id])?.first()
    {
        return Ok(row.id.clone());
    }
    let user_id = uuid::Uuid::now_v7().to_string();
    guard.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .aliases([alias_of(ENTITY_TYPE_USER, &user_id)])
            .values(UserRow {
                db_id: None,
                entity_type: ENTITY_TYPE_USER.to_string(),
                id: user_id.clone(),
                email_address_hash: email_address_hash.to_string(),
                name: user_id.replace('-', ""),
            })
            .query(),
    )?;
    Ok(user_id)
}

pub async fn read_user_by_email_address_hash(
    db: &DbHandle,
    email_address_hash: &str,
) -> Result<Option<String>, DbError> {
    let guard = db.read().await;
    let ids = find_by_index_sync(&guard, KEY_EMAIL_ADDRESS_HASH, email_address_hash)?;
    let Some(user_id) = ids.first() else {
        return Ok(None);
    };
    Ok(read_rows_sync::<IdRow>(&guard, &[*user_id])?
        .first()
        .map(|row| row.id.clone()))
}

pub async fn read_user(db: &DbHandle, user_id: &str) -> Result<Option<UserEntry>, DbError> {
    let guard = db.read().await;
    let Some(id) = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(None);
    };
    let row = read_rows_sync::<UserRow>(&guard, &[id])?
        .into_iter()
        .next()
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, "user row missing"))?;
    Ok(Some(UserEntry {
        email_address_hash: row.email_address_hash,
        name: row.name,
    }))
}

pub async fn read_user_names(
    db: &DbHandle,
    user_ids: &[String],
) -> Result<HashMap<String, String>, DbError> {
    let guard = db.read().await;
    let mut names = HashMap::new();
    for user_id in user_ids {
        let Some(node) = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)? else {
            continue;
        };
        if let Some(row) = read_rows_sync::<UserRow>(&guard, &[node])?
            .into_iter()
            .next()
        {
            names.insert(user_id.clone(), row.name);
        }
    }
    Ok(names)
}

pub async fn update_user_name(
    db: &DbHandle,
    user_id: &str,
    name: &str,
) -> Result<(), UserWriteError> {
    let mut guard = db.write().await;
    let id = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)
        .map_err(UserWriteError::Db)?
        .ok_or(UserWriteError::UserMissing)?;
    let taken = find_by_index_sync(&guard, KEY_USER_NAME, name)
        .map_err(UserWriteError::Db)?
        .into_iter()
        .any(|other_id| other_id != id);
    if taken {
        return Err(UserWriteError::AlreadyTaken);
    }
    guard
        .exec_mut(
            QueryBuilder::insert()
                .nodes()
                .ids([id])
                .values([[(KEY_USER_NAME, name).into()]])
                .query(),
        )
        .map_err(UserWriteError::Db)?;
    Ok(())
}

pub async fn update_user_email(
    db: &DbHandle,
    user_id: &str,
    old_email_address_hash: &str,
    new_email_address_hash: &str,
) -> Result<(), UserWriteError> {
    let mut guard = db.write().await;
    let id = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)
        .map_err(UserWriteError::Db)?
        .ok_or(UserWriteError::UserMissing)?;
    let current_hash = read_rows_sync::<UserRow>(&guard, &[id])
        .map_err(UserWriteError::Db)?
        .into_iter()
        .next()
        .map(|row| row.email_address_hash)
        .unwrap_or_default();
    if current_hash != old_email_address_hash {
        return Err(UserWriteError::EmailMismatch);
    }
    let taken = find_by_index_sync(&guard, KEY_EMAIL_ADDRESS_HASH, new_email_address_hash)
        .map_err(UserWriteError::Db)?
        .into_iter()
        .any(|other_id| other_id != id);
    if taken {
        return Err(UserWriteError::AlreadyTaken);
    }
    guard
        .exec_mut(
            QueryBuilder::insert()
                .nodes()
                .ids([id])
                .values([[(KEY_EMAIL_ADDRESS_HASH, new_email_address_hash).into()]])
                .query(),
        )
        .map_err(UserWriteError::Db)?;
    Ok(())
}

pub async fn read_users(db: &DbHandle) -> Result<Vec<UserListItem>, DbError> {
    let guard = db.read().await;
    let result = guard.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_USER)
            .query(),
    )?;
    let mut users = Vec::new();
    for element in &result.elements {
        if crate::repository::delete::has_soft_deleted_flag(&guard, element.id)? {
            continue;
        }
        if let Some(row) = read_rows_sync::<UserRow>(&guard, &[element.id])?
            .into_iter()
            .next()
        {
            users.push(UserListItem {
                id: row.id,
                name: row.name,
            });
        }
    }
    users.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(users)
}
