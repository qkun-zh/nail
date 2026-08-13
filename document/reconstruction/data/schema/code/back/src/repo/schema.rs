
use agdb::QueryBuilder;
use anyhow::Context;

use crate::repo::db::DbHandle;
use crate::repo::types::{
    KEY_CONTENT_HASH, KEY_EMAIL_ADDRESS_HASH, KEY_PERMISSION_NAME, KEY_ROLE_NAME, KEY_TAG_NAME,
    KEY_TITLE, KEY_USER_NAME,
};

pub async fn init_graph(db: &DbHandle, user_zero_email: &str) -> anyhow::Result<()> {
    {
        let mut db_guard = db.write().await;
        let existing = crate::repo::db::existing_index_keys(&db_guard)?;
        for key in [
            KEY_EMAIL_ADDRESS_HASH,
            KEY_USER_NAME,
            KEY_TITLE,
            KEY_CONTENT_HASH,
            KEY_TAG_NAME,
            KEY_ROLE_NAME,
            KEY_PERMISSION_NAME,
        ] {
            if existing.contains(key) {
                continue;
            }
            db_guard
                .exec_mut(QueryBuilder::insert().index(key).query())
                .with_context(|| format!("create index {key}"))?;
        }
    }

    crate::repo::authorization::seed_permissions(db)
        .await
        .with_context(|| "seed permission points")?;
    let user_zero_hash = common::hash::email(user_zero_email);
    let user_zero_id = crate::repo::user::find_or_create_user(db, &user_zero_hash)
        .await
        .with_context(|| "seed user zero")?;
    crate::repo::authorization::seed_user_zero_roles(db, &user_zero_id)
        .await
        .with_context(|| "seed required roles to user zero")?;
    Ok(())
}
