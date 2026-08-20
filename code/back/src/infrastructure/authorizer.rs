use std::sync::Arc;

use anyhow::Context;
use cedar_policy::{
    Authorizer as CedarAuthorizer, Decision, Entities, Entity, EntityUid, PolicySet, Request,
    ValidationMode, Validator,
};

use crate::repository::authorization::assemble;
use crate::repository::graph::DbHandle;

use super::cedar::{POLICY, SCHEMA};

#[derive(Clone)]
pub struct Authorizer {
    cedar: CedarAuthorizer,
    policies: Arc<PolicySet>,
    graph: DbHandle,
}

#[derive(Debug)]
pub enum AuthorizationError {
    Denied,
    ResourceNotFound,
    Internal(String),
}

impl std::fmt::Display for AuthorizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Denied => write!(f, "access denied"),
            Self::ResourceNotFound => write!(f, "resource not found"),
            Self::Internal(msg) => write!(f, "authorization error: {msg}"),
        }
    }
}

impl std::error::Error for AuthorizationError {}

impl From<anyhow::Error> for AuthorizationError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<crate::repository::authorization::AssemblyError> for AuthorizationError {
    fn from(error: crate::repository::authorization::AssemblyError) -> Self {
        match error {
            crate::repository::authorization::AssemblyError::ResourceNotFound => {
                Self::ResourceNotFound
            }
            crate::repository::authorization::AssemblyError::Internal(msg) => Self::Internal(msg),
        }
    }
}

impl Authorizer {
    pub fn new(graph: DbHandle) -> Result<Self, AuthorizationError> {
        let policies = POLICY
            .parse::<PolicySet>()
            .map_err(|error| AuthorizationError::Internal(error.to_string()))?;

        let schema = SCHEMA
            .parse::<cedar_policy::Schema>()
            .map_err(|error| AuthorizationError::Internal(error.to_string()))?;

        let validation = Validator::new(schema).validate(&policies, ValidationMode::Strict);
        if !validation.validation_passed() {
            let messages: Vec<String> = validation
                .validation_errors()
                .map(std::string::ToString::to_string)
                .collect();
            return Err(AuthorizationError::Internal(format!(
                "policy does not validate against schema: {messages:?}"
            )));
        }

        Ok(Self {
            cedar: CedarAuthorizer::new(),
            policies: Arc::new(policies),
            graph,
        })
    }

    pub async fn authorize(
        &self,
        user_id: &str,
        action: &str,
        resource: &crate::repository::authorization::Resource,
    ) -> Result<(), AuthorizationError> {
        let assembly = assemble(&self.graph, user_id, resource.clone()).await?;

        let action_uid = action_uid(action)?;
        let mut entities = assembly.entities;

        if !entities
            .iter()
            .any(|entity| entity.uid() == action_uid.clone())
        {
            entities.push(Entity::new_no_attrs(
                action_uid.clone(),
                std::collections::HashSet::default(),
            ));
        }

        let entities = Entities::from_entities(entities, None)
            .map_err(|error| AuthorizationError::Internal(error.to_string()))?;

        let request = Request::new(
            assembly.principal,
            action_uid,
            assembly.resource,
            cedar_policy::Context::empty(),
            None,
        )
        .map_err(|error| AuthorizationError::Internal(error.to_string()))?;

        match self
            .cedar
            .is_authorized(&request, &self.policies, &entities)
            .decision()
        {
            Decision::Allow => Ok(()),
            Decision::Deny => Err(AuthorizationError::Denied),
        }
    }
}

fn action_uid(action: &str) -> Result<EntityUid, AuthorizationError> {
    format!("Action::\"{action}\"")
        .parse::<EntityUid>()
        .with_context(|| format!("invalid action uid for {action:?}"))
        .map_err(AuthorizationError::from)
}
