use agdb::QueryBuilder;
use anyhow::Context;

use crate::repository::graph::{DbHandle, existing_index_keys};
use crate::repository::role::{
    ALL_PERMISSIONS, PERMISSION_ARTICLE_CREATE, PERMISSION_COMMENT_CREATE, REQUIRED_ROLES,
    ROLE_ADMIN, ROLE_MEMBER,
};
use crate::repository::schema::{
    KEY_CONTENT_HASH, KEY_EMAIL_ADDRESS_HASH, KEY_PERMISSION_NAME, KEY_ROLE_NAME, KEY_TAG_NAME,
    KEY_TITLE, KEY_USER_NAME,
};
use crate::repository::user::find_or_create_user;

pub async fn init_graph(db: &DbHandle, user_zero_email: &str) -> anyhow::Result<()> {
    create_indexes(db).await?;
    seed_roles_and_permissions(db).await?;
    let user_zero_hash = nail_common::hash::email(user_zero_email);
    let user_zero_id = find_or_create_user(db, &user_zero_hash)
        .await
        .with_context(|| "seed user zero")?;
    for role_name in REQUIRED_ROLES {
        crate::repository::role::hold_role(db, &user_zero_id, role_name).await?;
    }
    Ok(())
}

async fn create_indexes(db: &DbHandle) -> anyhow::Result<()> {
    let mut guard = db.write().await;
    let existing = existing_index_keys(&guard)?;
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
        guard
            .exec_mut(QueryBuilder::insert().index(key).query())
            .with_context(|| format!("create index {key}"))?;
    }
    Ok(())
}

async fn seed_roles_and_permissions(db: &DbHandle) -> anyhow::Result<()> {
    for role_name in REQUIRED_ROLES {
        crate::repository::role::create_role(db, role_name).await?;
    }
    for permission_name in ALL_PERMISSIONS {
        crate::repository::role::create_permission(db, permission_name).await?;
    }
    for permission_name in ALL_PERMISSIONS {
        crate::repository::role::grant_permission_to_role(db, ROLE_ADMIN, permission_name).await?;
    }
    crate::repository::role::grant_permission_to_role(db, ROLE_MEMBER, PERMISSION_ARTICLE_CREATE)
        .await?;
    crate::repository::role::grant_permission_to_role(db, ROLE_MEMBER, PERMISSION_COMMENT_CREATE)
        .await?;
    Ok(())
}
