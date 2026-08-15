
use agdb::{DbError, QueryBuilder};

use crate::repo::db::{DbHandle, find_by_index_sync, read_rows_sync, resolve_node_id_sync};
use crate::repo::types::{
    ENTITY_TYPE_USER, IdRow, KEY_EMAIL_ADDRESS_HASH, KEY_TYPE, KEY_USER_NAME, UserEntry, UserRow,
    alias_of,
};

pub async fn find_user_by_email_address_hash(
    db: &DbHandle,
    email_address_hash: &str,
) -> Result<Option<String>, DbError> {
    let db = db.read().await;
    let ids = find_by_index_sync(&db, KEY_EMAIL_ADDRESS_HASH, email_address_hash)?;
    let Some(user_id) = ids.first() else {
        return Ok(None);
    };
    Ok(crate::repo::db::read_rows_sync::<IdRow>(&db, &[*user_id])?
        .first()
        .map(|r| r.id.clone()))
}

#[allow(dead_code)]
pub async fn create_user(
    db: &DbHandle,
    user_id: &str,
    email_address_hash: &str,
) -> Result<(), DbError> {
    let mut db = db.write().await;
    db.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .aliases([alias_of(ENTITY_TYPE_USER, user_id)])
            .values(UserRow {
                db_id: None,
                entity_type: ENTITY_TYPE_USER.to_string(),
                id: user_id.to_string(),
                email_address_hash: email_address_hash.to_string(),
                name: user_id.replace('-', ""),
            })
            .query(),
    )?;
    Ok(())
}

pub async fn find_or_create_user(
    db: &DbHandle,
    email_address_hash: &str,
) -> Result<String, DbError> {
    let mut db = db.write().await;
    let ids = find_by_index_sync(&db, KEY_EMAIL_ADDRESS_HASH, email_address_hash)?;
    if let Some(user_id) = ids.first()
        && let Some(row) = read_rows_sync::<IdRow>(&db, &[*user_id])?.first()
    {
        return Ok(row.id.clone());
    }
    let user_id = uuid::Uuid::now_v7().to_string();
    db.exec_mut(
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

pub async fn read_user(db: &DbHandle, user_id: &str) -> Result<Option<UserEntry>, DbError> {
    let db = db.read().await;
    let Some(id) = resolve_node_id_sync(&db, ENTITY_TYPE_USER, user_id)? else {
        return Ok(None);
    };
    let row = read_rows_sync::<UserRow>(&db, &[id])?
        .into_iter()
        .next()
        .ok_or_else(|| DbError::query(agdb::DbErrorType::NotFound, "user row missing"))?;
    Ok(Some(UserEntry {
        email_address_hash: row.email_address_hash,
        name: row.name,
    }))
}

pub struct UserListItem {
    pub id: String,
    pub name: String,
    pub email_address_hash: String,
}

pub async fn list_users(
    db: &DbHandle,
    limit: u64,
    offset: u64,
) -> Result<(Vec<UserListItem>, u64), DbError> {
    let db = db.read().await;
    let result = db.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(ENTITY_TYPE_USER)
            .query(),
    )?;
    let mut users = Vec::new();
    for el in &result.elements {
        if let Ok(rows) = read_rows_sync::<UserRow>(&db, &[el.id]) {
            if let Some(row) = rows.into_iter().next() {
                users.push(UserListItem {
                    id: row.id,
                    name: row.name,
                    email_address_hash: row.email_address_hash,
                });
            }
        }
    }
    users.sort_by(|a, b| b.id.cmp(&a.id));
    let total = users.len() as u64;
    let page: Vec<UserListItem> = users
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();
    Ok((page, total))
}

pub async fn read_user_names_by_ids(
    db: &DbHandle,
    user_ids: &[String],
) -> Result<Vec<(String, String)>, DbError> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    let db = db.read().await;
    let mut out = Vec::with_capacity(user_ids.len());
    for user_id in user_ids {
        let name = match resolve_node_id_sync(&db, ENTITY_TYPE_USER, user_id)? {
            Some(id) => read_rows_sync::<UserRow>(&db, &[id])?
                .into_iter()
                .next()
                .map(|row| row.name)
                .unwrap_or_default(),
            None => String::new(),
        };
        out.push((user_id.clone(), name));
    }
    Ok(out)
}

#[derive(Debug)]
pub enum UserWriteError {
    UserMissing,
    AlreadyTaken,
    Db(DbError),
}

impl std::fmt::Display for UserWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserWriteError::UserMissing => write!(f, "user not found"),
            UserWriteError::AlreadyTaken => write!(f, "value already taken"),
            UserWriteError::Db(e) => write!(f, "database query failed: {e}"),
        }
    }
}
impl std::error::Error for UserWriteError {}

pub async fn update_user_email(
    db: &DbHandle,
    user_id: &str,
    old_email_address_hash: &str,
    new_email_address_hash: &str,
) -> Result<bool, UserWriteError> {
    let mut db = db.write().await;
    let id = resolve_node_id_sync(&db, ENTITY_TYPE_USER, user_id)
        .map_err(UserWriteError::Db)?
        .ok_or(UserWriteError::UserMissing)?;
    let current = read_rows_sync::<UserRow>(&db, &[id])
        .map_err(UserWriteError::Db)?
        .into_iter()
        .next()
        .map(|row| row.email_address_hash)
        .unwrap_or_default();
    if current != old_email_address_hash {
        return Ok(false);
    }
    let taken = find_by_index_sync(&db, KEY_EMAIL_ADDRESS_HASH, new_email_address_hash)
        .map_err(UserWriteError::Db)?
        .into_iter()
        .any(|other_id| other_id != id);
    if taken {
        return Err(UserWriteError::AlreadyTaken);
    }
    db.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([id])
            .values([[(KEY_EMAIL_ADDRESS_HASH, new_email_address_hash).into()]])
            .query(),
    )
    .map_err(UserWriteError::Db)?;
    Ok(true)
}

pub async fn update_user_name(
    db: &DbHandle,
    user_id: &str,
    name: &str,
) -> Result<bool, UserWriteError> {
    let mut db = db.write().await;
    let id = resolve_node_id_sync(&db, ENTITY_TYPE_USER, user_id)
        .map_err(UserWriteError::Db)?
        .ok_or(UserWriteError::UserMissing)?;
    let taken = find_by_index_sync(&db, KEY_USER_NAME, name)
        .map_err(UserWriteError::Db)?
        .into_iter()
        .any(|other_id| other_id != id);
    if taken {
        return Err(UserWriteError::AlreadyTaken);
    }
    db.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .ids([id])
            .values([[(KEY_USER_NAME, name).into()]])
            .query(),
    )
    .map_err(UserWriteError::Db)?;
    Ok(true)
}
