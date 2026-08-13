use agdb::{DbError, QueryBuilder};

use crate::repository::graph::{DbHandle, find_by_index_sync, read_rows_sync, resolve_node_id_sync};
use crate::repository::schema::{
    ENTITY_TYPE_USER, KEY_EMAIL_ADDRESS_HASH, IdRow, UserRow, alias_of,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserEntry {
    pub email_address_hash: String,
    pub name: String,
}

pub async fn find_or_create_user(
    db: &DbHandle,
    email_address_hash: &str,
) -> Result<String, DbError> {
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

pub async fn find_user_by_email_address_hash(
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
