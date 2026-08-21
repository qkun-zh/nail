use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use cedar_policy::{
    Authorizer as CedarAuthorizer, Decision, Entities, Entity, EntityUid, PolicySet, Request,
    RestrictedExpression, ValidationMode, Validator,
};

use crate::error::Error;
use crate::principal::Principal;
use crate::resource::Resource;

const POLICY: &str = include_str!("../cedar/policy.cedar");
const SCHEMA: &str = include_str!("../cedar/schema.cedar");

include!(concat!(env!("OUT_DIR"), "/cedar_entities.rs"));

#[derive(Clone)]
pub struct Authorizer {
    cedar: Arc<CedarAuthorizer>,
    policies: Arc<PolicySet>,
}

impl Authorizer {
    /// Creates a validated authorizer. Parses the embedded policy and schema
    /// once; strict validation failure is an `Internal` error.
    ///
    /// # Errors
    /// Returns `Error::Internal` when the embedded policy or schema is malformed
    /// or the policy does not validate against the schema.
    pub fn new() -> Result<Self, Error> {
        let policies = POLICY
            .parse::<PolicySet>()
            .map_err(|error| Error::Internal(error.to_string()))?;
        let schema = SCHEMA
            .parse::<cedar_policy::Schema>()
            .map_err(|error| Error::Internal(error.to_string()))?;
        let validation = Validator::new(schema).validate(&policies, ValidationMode::Strict);
        if !validation.validation_passed() {
            let messages: Vec<String> = validation
                .validation_errors()
                .map(std::string::ToString::to_string)
                .collect();
            return Err(Error::Internal(format!(
                "policy does not validate against schema: {messages:?}"
            )));
        }
        Ok(Self {
            cedar: Arc::new(CedarAuthorizer::new()),
            policies: Arc::new(policies),
        })
    }

    /// Authorizes `principal` performing `action` on `resource`.
    ///
    /// # Errors
    /// Returns `Error::Denied` for a Cedar `Deny`, `Error::NotFound` is never
    /// produced here (resource existence is the adapter's concern), and
    /// `Error::Internal` for malformed UIDs or Cedar construction failures.
    pub fn authorize(
        &self,
        principal: &Principal,
        action: &str,
        resource: &Resource,
    ) -> Result<(), Error> {
        let (principal_uid, principal_entities) = build_principal(principal)?;
        let (resource_uid, resource_entities) = build_resource(resource)?;
        let mut entities = merge_entities(principal_entities, resource_entities);

        let action_uid = action_uid(action)?;
        if !entities.iter().any(|entity| entity.uid() == action_uid) {
            entities.push(Entity::new_no_attrs(action_uid.clone(), HashSet::new()));
        }

        let entities = Entities::from_entities(entities, None)
            .map_err(|error| Error::Internal(error.to_string()))?;
        let request = Request::new(
            principal_uid,
            action_uid,
            resource_uid,
            cedar_policy::Context::empty(),
            None,
        )
        .map_err(|error| Error::Internal(error.to_string()))?;

        match self
            .cedar
            .is_authorized(&request, &self.policies, &entities)
            .decision()
        {
            Decision::Allow => Ok(()),
            Decision::Deny => Err(Error::Denied),
        }
    }
}

fn merge_entities(mut principal: Vec<Entity>, resource: Vec<Entity>) -> Vec<Entity> {
    let mut positions: HashMap<EntityUid, usize> = HashMap::new();
    let mut merged: Vec<Entity> =
        Vec::with_capacity(principal.len().saturating_add(resource.len()));
    for entity in principal.drain(..).chain(resource) {
        if let Some(index) = positions.get(&entity.uid()) {
            merged[*index] = entity;
        } else {
            positions.insert(entity.uid().clone(), merged.len());
            merged.push(entity);
        }
    }
    merged
}

fn build_principal(principal: &Principal) -> Result<(EntityUid, Vec<Entity>), Error> {
    let uid = user_uid(&principal.id)?;
    let mut entities = Vec::new();
    let mut seen = HashSet::new();
    let mut role_uids = HashSet::new();

    for role in &principal.roles {
        let role_uid = role_uid(&role.name)?;
        role_uids.insert(role_uid.clone());
        let mut action_parents = HashSet::new();
        for permission in &role.permissions {
            let action = action_uid(permission)?;
            action_parents.insert(action.clone());
            if seen.insert(action.clone()) {
                entities.push(Entity::new_no_attrs(action, HashSet::new()));
            }
        }
        if seen.insert(role_uid.clone()) {
            entities.push(Entity::new_no_attrs(role_uid, action_parents));
        }
    }

    entities.push(Entity::new_no_attrs(uid.clone(), role_uids));
    Ok((uid, entities))
}

fn build_resource(resource: &Resource) -> Result<(EntityUid, Vec<Entity>), Error> {
    match resource {
        Resource::Article { id, owner } => {
            let resource_uid = article_uid(id)?;
            let entity = Entity::new(resource_uid.clone(), resource_attrs(owner)?, HashSet::new())
                .map_err(|error| Error::Internal(error.to_string()))?;
            Ok((resource_uid, vec![entity]))
        }
        Resource::Version {
            id,
            article_id,
            owner,
        } => {
            let article_entity_uid = article_uid(article_id)?;
            let version_entity_uid = version_uid(id)?;
            let article_entity = Entity::new(
                article_entity_uid.clone(),
                resource_attrs(owner)?,
                HashSet::new(),
            )
            .map_err(|error| Error::Internal(error.to_string()))?;
            let version_entity = Entity::new(
                version_entity_uid.clone(),
                resource_attrs(owner)?,
                HashSet::from([article_entity_uid]),
            )
            .map_err(|error| Error::Internal(error.to_string()))?;
            Ok((version_entity_uid, vec![article_entity, version_entity]))
        }
        Resource::Comment {
            id,
            version_id,
            article_id,
            article_owner,
            owner,
        } => {
            let article_entity_uid = article_uid(article_id)?;
            let version_entity_uid = version_uid(version_id)?;
            let comment_entity_uid = comment_uid(id)?;
            let article_entity = Entity::new(
                article_entity_uid.clone(),
                resource_attrs(article_owner)?,
                HashSet::new(),
            )
            .map_err(|error| Error::Internal(error.to_string()))?;
            let version_entity = Entity::new(
                version_entity_uid.clone(),
                resource_attrs(article_owner)?,
                HashSet::from([article_entity_uid]),
            )
            .map_err(|error| Error::Internal(error.to_string()))?;
            let comment_entity = Entity::new(
                comment_entity_uid.clone(),
                resource_attrs(owner)?,
                HashSet::from([version_entity_uid]),
            )
            .map_err(|error| Error::Internal(error.to_string()))?;
            Ok((
                comment_entity_uid,
                vec![article_entity, version_entity, comment_entity],
            ))
        }
        Resource::Role { name, permissions } => {
            let resource_uid = role_uid(name)?;
            let action_parents = permissions
                .iter()
                .map(|permission| action_uid(permission))
                .collect::<Result<HashSet<_>, _>>()?;
            let entity = Entity::new_no_attrs(resource_uid.clone(), action_parents);
            Ok((resource_uid, vec![entity]))
        }
        Resource::User(user_id) => {
            let resource_uid = user_uid(user_id)?;
            let entity = Entity::new_no_attrs(resource_uid.clone(), HashSet::new());
            Ok((resource_uid, vec![entity]))
        }
        Resource::Tag(tag_id) => {
            let resource_uid = tag_uid(tag_id)?;
            let entity = Entity::new_no_attrs(resource_uid.clone(), HashSet::new());
            Ok((resource_uid, vec![entity]))
        }
        Resource::Virtual(name) => {
            let resource_uid = virtual_uid(name)?;
            let entity = Entity::new_no_attrs(resource_uid.clone(), HashSet::new());
            Ok((resource_uid, vec![entity]))
        }
    }
}

fn parse_uid(text: &str) -> Result<EntityUid, Error> {
    text.parse::<EntityUid>()
        .map_err(|error| Error::Internal(format!("invalid entity uid {text:?}: {error}")))
}

fn user_uid(user_id: &str) -> Result<EntityUid, Error> {
    parse_uid(&format!("{CEDAR_ENTITY_USER}::\"{user_id}\""))
}

fn role_uid(role_name: &str) -> Result<EntityUid, Error> {
    parse_uid(&format!("{CEDAR_ENTITY_ROLE}::\"{role_name}\""))
}

fn action_uid(action: &str) -> Result<EntityUid, Error> {
    parse_uid(&format!("Action::\"{action}\""))
}

fn article_uid(article_id: &str) -> Result<EntityUid, Error> {
    parse_uid(&format!("{CEDAR_ENTITY_ARTICLE}::\"{article_id}\""))
}

fn version_uid(version_id: &str) -> Result<EntityUid, Error> {
    parse_uid(&format!("{CEDAR_ENTITY_VERSION}::\"{version_id}\""))
}

fn comment_uid(comment_id: &str) -> Result<EntityUid, Error> {
    parse_uid(&format!("{CEDAR_ENTITY_COMMENT}::\"{comment_id}\""))
}

fn tag_uid(tag_id: &str) -> Result<EntityUid, Error> {
    parse_uid(&format!("{CEDAR_ENTITY_TAG}::\"{tag_id}\""))
}

fn virtual_uid(name: &str) -> Result<EntityUid, Error> {
    parse_uid(&format!("{CEDAR_ENTITY_VIRTUAL}::\"{name}\""))
}

fn expression(text: &str) -> Result<RestrictedExpression, Error> {
    RestrictedExpression::from_str(text)
        .map_err(|error| Error::Internal(format!("invalid expression {text:?}: {error}")))
}

fn resource_attrs(owner_id: &str) -> Result<HashMap<String, RestrictedExpression>, Error> {
    let owner = if owner_id.is_empty() {
        expression("User::\"\"")?
    } else {
        expression(&user_uid(owner_id)?.to_string())?
    };
    Ok(HashMap::from([("owner".to_string(), owner)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::principal::Role;

    fn principal_with_roles(id: &str, roles: Vec<(&str, Vec<&str>)>) -> Principal {
        Principal {
            id: id.to_string(),
            roles: roles
                .into_iter()
                .map(|(name, perms)| Role {
                    name: name.to_string(),
                    permissions: perms.into_iter().map(ToString::to_string).collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn new_validates_policy() {
        assert!(Authorizer::new().is_ok());
    }

    #[test]
    fn member_article_create_on_virtual_allow() {
        let authorizer = Authorizer::new().expect("authorizer");
        let principal = principal_with_roles("alice", vec![("member", vec!["Article::Create"])]);
        let resource = Resource::Virtual("any".to_string());
        assert!(
            authorizer
                .authorize(&principal, "Article::Create", &resource)
                .is_ok()
        );
    }

    #[test]
    fn owner_bypass_allow_without_role() {
        let authorizer = Authorizer::new().expect("authorizer");
        let principal = Principal {
            id: "alice".to_string(),
            roles: vec![],
        };
        let resource = Resource::Article {
            id: "a1".to_string(),
            owner: "alice".to_string(),
        };
        assert!(
            authorizer
                .authorize(&principal, "Article::Update", &resource)
                .is_ok()
        );
    }

    #[test]
    fn non_owner_denied_without_grant() {
        let authorizer = Authorizer::new().expect("authorizer");
        let principal = Principal {
            id: "bob".to_string(),
            roles: vec![],
        };
        let resource = Resource::Article {
            id: "a1".to_string(),
            owner: "alice".to_string(),
        };
        assert_eq!(
            authorizer.authorize(&principal, "Article::Update", &resource),
            Err(Error::Denied)
        );
    }
}
