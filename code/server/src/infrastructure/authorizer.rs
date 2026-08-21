use authorizer::{Authorizer as InnerAuthorizer, Principal, Resource as AuthResource, Role};
use database::Database;

#[derive(Clone)]
pub struct Authorizer {
    inner: InnerAuthorizer,
    graph: Database,
}

#[derive(Debug)]
pub enum AuthorizationError {
    Denied,
    ResourceNotFound,
    Internal(String),
}

impl std::fmt::Display for AuthorizationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Denied => formatter.write_str("access denied"),
            Self::ResourceNotFound => formatter.write_str("resource not found"),
            Self::Internal(message) => write!(formatter, "authorization error: {message}"),
        }
    }
}

impl std::error::Error for AuthorizationError {}

impl From<authorizer::Error> for AuthorizationError {
    fn from(error: authorizer::Error) -> Self {
        match error {
            authorizer::Error::Denied => Self::Denied,
            authorizer::Error::NotFound => Self::ResourceNotFound,
            authorizer::Error::Internal(message) => Self::Internal(message),
        }
    }
}

impl Authorizer {
    pub fn new(graph: Database) -> Result<Self, AuthorizationError> {
        let inner = InnerAuthorizer::new().map_err(AuthorizationError::from)?;
        Ok(Self { inner, graph })
    }

    pub fn authorize(
        &self,
        user_id: &str,
        action: &str,
        resource: &crate::repository::authorization::Resource,
    ) -> Result<(), AuthorizationError> {
        let principal = build_principal(&self.graph, user_id)?;
        let auth_resource = build_resource(&self.graph, resource)?;
        self.inner
            .authorize(&principal, action, &auth_resource)
            .map_err(AuthorizationError::from)
    }
}

fn build_principal(database: &Database, user_id: &str) -> Result<Principal, AuthorizationError> {
    let authorization =
        crate::repository::authorization::read_user_authorization(database, user_id)
            .map_err(|error| AuthorizationError::Internal(error.to_string()))?;
    let roles = authorization
        .roles
        .into_iter()
        .map(|view| Role {
            name: view.role_name,
            permissions: view.permissions,
        })
        .collect();
    Ok(Principal {
        id: user_id.to_string(),
        roles,
    })
}

fn build_resource(
    database: &Database,
    resource: &crate::repository::authorization::Resource,
) -> Result<AuthResource, AuthorizationError> {
    match resource {
        crate::repository::authorization::Resource::Article(article_id) => {
            let authorization =
                crate::repository::authorization::read_article_authorization(database, article_id)
                    .map_err(|error| AuthorizationError::Internal(error.to_string()))?
                    .ok_or(AuthorizationError::ResourceNotFound)?;
            Ok(AuthResource::Article {
                id: article_id.clone(),
                owner: authorization.owner_id,
            })
        }
        crate::repository::authorization::Resource::Version(version_id) => {
            let article_id = crate::repository::version::parent_article_of(database, version_id)
                .map_err(|error| AuthorizationError::Internal(error.to_string()))?
                .ok_or(AuthorizationError::ResourceNotFound)?;
            let authorization =
                crate::repository::authorization::read_article_authorization(database, &article_id)
                    .map_err(|error| AuthorizationError::Internal(error.to_string()))?
                    .ok_or(AuthorizationError::ResourceNotFound)?;
            Ok(AuthResource::Version {
                id: version_id.clone(),
                article_id,
                owner: authorization.owner_id,
            })
        }
        crate::repository::authorization::Resource::Comment(comment_id) => {
            let version_id = crate::repository::comment::version_of_comment(database, comment_id)
                .map_err(|error| AuthorizationError::Internal(error.to_string()))?
                .ok_or(AuthorizationError::ResourceNotFound)?;
            let article_id = crate::repository::version::parent_article_of(database, &version_id)
                .map_err(|error| AuthorizationError::Internal(error.to_string()))?
                .ok_or(AuthorizationError::ResourceNotFound)?;
            let article_authorization =
                crate::repository::authorization::read_article_authorization(database, &article_id)
                    .map_err(|error| AuthorizationError::Internal(error.to_string()))?
                    .ok_or(AuthorizationError::ResourceNotFound)?;
            let owner = crate::repository::comment::owner_of_comment(database, comment_id)
                .map_err(|error| AuthorizationError::Internal(error.to_string()))?
                .unwrap_or_default();
            Ok(AuthResource::Comment {
                id: comment_id.clone(),
                version_id,
                article_id,
                article_owner: article_authorization.owner_id,
                owner,
            })
        }
        crate::repository::authorization::Resource::Role(name) => {
            let view = crate::repository::role::read_role(database, name)
                .map_err(|error| AuthorizationError::Internal(error.to_string()))?
                .ok_or(AuthorizationError::ResourceNotFound)?;
            Ok(AuthResource::Role {
                name: name.clone(),
                permissions: view.permissions,
            })
        }
        crate::repository::authorization::Resource::User(user_id) => {
            let exists = database
                .read(|scope| Ok(scope.resolve(database::NodeKind::User, user_id)?.is_some()))
                .map_err(|error: database::Error| {
                    AuthorizationError::Internal(error.to_string())
                })?;
            if !exists {
                return Err(AuthorizationError::ResourceNotFound);
            }
            Ok(AuthResource::User(user_id.clone()))
        }
        crate::repository::authorization::Resource::Tag(tag_id) => {
            let exists = database
                .read(|scope| Ok(scope.resolve(database::NodeKind::Tag, tag_id)?.is_some()))
                .map_err(|error: database::Error| {
                    AuthorizationError::Internal(error.to_string())
                })?;
            if !exists {
                return Err(AuthorizationError::ResourceNotFound);
            }
            Ok(AuthResource::Tag(tag_id.clone()))
        }
        crate::repository::authorization::Resource::Virtual(name) => {
            Ok(AuthResource::Virtual(name.clone()))
        }
    }
}
