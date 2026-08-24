use database::{Database, EdgeKind, Error, NodeKind};

use crate::repository::access::GraphRead;
use crate::repository::role::RoleView;
use crate::repository::schema::{IdRow, PermissionRow, RoleRow};
use authorizer::{ALL_PERMISSIONS, Grant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    Article(String),
    Version(String),
    Comment(String),
    Role(String),
    User(String),
    Tag(String),
    Virtual(String),
}

#[derive(Debug, Clone, Default)]
pub struct UserAuthorization {
    pub roles: Vec<RoleView>,
}

#[derive(Debug, Clone, Default)]
pub struct ArticleAuthorization {
    pub owner_id: String,
}

pub fn read_user_authorization(db: &Database, user_id: &str) -> Result<UserAuthorization, Error> {
    db.read(|scope| {
        let Some(user) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(UserAuthorization::default());
        };
        let role_edges = scope.outgoing(user, EdgeKind::UserHoldRole)?;
        let mut authorization = UserAuthorization::default();
        for role_node in role_edges {
            let Some(row) = scope.scope_read_node::<RoleRow>(role_node)? else {
                continue;
            };
            let mut role = RoleView {
                role_name: row.role_name,
                ..Default::default()
            };
            let grants = scope.outgoing(role_node, EdgeKind::RoleGrantPermission)?;
            let permissions = scope.scope_read_nodes::<PermissionRow>(&grants)?;
            role.permissions
                .extend(permissions.into_iter().map(|p| p.permission_name));
            authorization.roles.push(role);
        }
        Ok(authorization)
    })
}

pub fn read_article_authorization(
    db: &Database,
    article_id: &str,
) -> Result<Option<ArticleAuthorization>, Error> {
    db.read(|scope| {
        let Some(article) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(None);
        };
        let mut authorization = ArticleAuthorization::default();
        if let Some(owner) = scope
            .incoming(article, EdgeKind::UserAuthorArticle)?
            .first()
            && let Some(row) = scope.scope_read_node::<IdRow>(*owner)?
        {
            authorization.owner_id = row.id;
        }
        Ok(Some(authorization))
    })
}

/// Reads every role-to-permission edge as a Cedar [`Grant`]. Permission nodes
/// form a closed vocabulary seeded from the schema actions, so iterating the
/// vocabulary and pulling incoming grant edges covers all grants.
pub fn read_all_role_grants(db: &Database) -> Result<Vec<Grant>, Error> {
    db.read(|scope| {
        let mut grants = Vec::new();
        for permission in ALL_PERMISSIONS {
            let Some(node) = scope.resolve(NodeKind::Permission, permission)? else {
                continue;
            };
            for role_node in scope.incoming(node, EdgeKind::RoleGrantPermission)? {
                let Some(row) = scope.scope_read_node::<RoleRow>(role_node)? else {
                    continue;
                };
                grants.push(Grant {
                    role: row.role_name,
                    permission: (*permission).to_string(),
                });
            }
        }
        Ok(grants)
    })
}
