use anyhow::Context;

use database::Database;

use crate::repository::role::{
    PERMISSION_ARTICLE_CREATE, PERMISSION_ARTICLE_READ, PERMISSION_COMMENT_CREATE,
    PERMISSION_COMMENT_READ, PERMISSION_VERSION_READ, REQUIRED_ROLES, ROLE_ADMIN, ROLE_MEMBER,
};
use crate::repository::user::create_user;

pub fn init_graph(db: &Database, user_zero_email: &str) -> anyhow::Result<()> {
    seed_roles_and_permissions(db)?;
    let user_zero_hash = common::hash::hash(user_zero_email.as_bytes())?;
    let user_zero_id = create_user(db, &user_zero_hash).with_context(|| "seed user zero")?;
    for role_name in REQUIRED_ROLES {
        crate::repository::role::hold_role(db, &user_zero_id, role_name)?;
    }
    Ok(())
}

fn seed_roles_and_permissions(db: &Database) -> anyhow::Result<()> {
    for role_name in REQUIRED_ROLES {
        crate::repository::role::create_role(db, role_name)?;
    }
    for permission_name in authorizer::ALL_PERMISSIONS {
        crate::repository::role::create_permission(db, permission_name)?;
    }
    for permission_name in authorizer::ALL_PERMISSIONS {
        crate::repository::role::grant_permission_to_role(db, ROLE_ADMIN, permission_name)?;
    }
    crate::repository::role::grant_permission_to_role(db, ROLE_MEMBER, PERMISSION_ARTICLE_CREATE)?;
    crate::repository::role::grant_permission_to_role(db, ROLE_MEMBER, PERMISSION_COMMENT_CREATE)?;
    crate::repository::role::grant_permission_to_role(db, ROLE_MEMBER, PERMISSION_ARTICLE_READ)?;
    crate::repository::role::grant_permission_to_role(db, ROLE_MEMBER, PERMISSION_VERSION_READ)?;
    crate::repository::role::grant_permission_to_role(db, ROLE_MEMBER, PERMISSION_COMMENT_READ)?;
    Ok(())
}
