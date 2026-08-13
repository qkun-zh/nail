
use cedar_policy::{Authorizer, Context, Decision, Entities, PolicySet, Request};

use crate::authorization::entity_store::{AssemblyError, assemble, assemble_principal};
use crate::authorization::entity_store::{AuthAssembly, Resource};
use crate::logic::error::LogicError;
use crate::other::AppState;

#[allow(unused_imports)]
pub use crate::repo::authorization::{
    PERMISSION_ARTICLE_CREATE, PERMISSION_ARTICLE_DELETE, PERMISSION_ARTICLE_READ,
    PERMISSION_ARTICLE_UPDATE, PERMISSION_COMMENT_CREATE, PERMISSION_COMMENT_DELETE,
    PERMISSION_COMMENT_READ, PERMISSION_COMMENT_UPDATE, PERMISSION_ROLE_MANAGE,
    PERMISSION_USER_DELETE, PERMISSION_USER_READ, PERMISSION_USER_UPDATE,
    PERMISSION_VERSION_CREATE, PERMISSION_VERSION_DELETE, PERMISSION_VERSION_READ,
    PERMISSION_VERSION_UPDATE,
};

pub async fn authorize(
    state: &AppState,
    user_id: &str,
    action: &str,
    resource: &Resource,
) -> Result<(), LogicError> {
    let assembly = assemble(&state.db, user_id, resource.clone())
        .await
        .map_err(map_assembly_error)?;
    let decision = evaluate(&assembly, action)
        .map_err(|e| LogicError::internal(format!("authorization evaluation failed: {e}")))?;
    if decision == Decision::Allow {
        Ok(())
    } else {
        Err(LogicError::forbidden("you are denied"))
    }
}

pub async fn authorize_or(
    state: &AppState,
    user_id: &str,
    action: &str,
    resource: &Resource,
    not_found_message: &str,
) -> Result<(), LogicError> {
    match authorize(state, user_id, action, resource).await {
        Ok(()) => Ok(()),
        Err(LogicError::NotFound(_)) => Err(LogicError::not_found(not_found_message)),
        Err(error) => Err(error),
    }
}

pub async fn is_allowed(
    state: &AppState,
    user_id: &str,
    action: &str,
    resource: &Resource,
) -> bool {
    match assemble(&state.db, user_id, resource.clone()).await {
        Ok(assembly) => match evaluate(&assembly, action) {
            Ok(decision) => decision == Decision::Allow,
            Err(_) => false,
        },
        Err(AssemblyError::ResourceNotFound) => false,
        Err(AssemblyError::Internal(_)) => false,
    }
}

pub async fn authorize_create(
    state: &AppState,
    user_id: &str,
    action: &str,
) -> Result<(), LogicError> {
    let (principal, mut entities) = assemble_principal(&state.db, user_id)
        .await
        .map_err(map_assembly_error)?;
    let resource_uid: cedar_policy::EntityUid = "Article::\"__create__\""
        .parse()
        .map_err(|e| LogicError::internal(format!("invalid create resource uid: {e}")))?;
    let resource_entity = cedar_policy::Entity::new(
        resource_uid.clone(),
        std::collections::HashMap::new(),
        std::collections::HashSet::new(),
    )
    .map_err(|e| LogicError::internal(format!("failed to build create resource: {e}")))?;
    entities.push(resource_entity);
    let assembly = AuthAssembly {
        principal,
        resource: resource_uid,
        entities,
    };
    let decision = evaluate(&assembly, action)
        .map_err(|e| LogicError::internal(format!("authorization evaluation failed: {e}")))?;
    if decision == Decision::Allow {
        Ok(())
    } else {
        Err(LogicError::forbidden("you are denied"))
    }
}

fn evaluate(assembly: &AuthAssembly, action: &str) -> anyhow::Result<Decision> {
    let policies: PolicySet = crate::authorization::POLICY.parse()?;
    let mut entities_vec = assembly.entities.clone();
    let action_uid: cedar_policy::EntityUid = format!("Action::\"{action}\"")
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid action uid for {action:?}: {e}"))?;
    if !entities_vec.iter().any(|e| e.uid() == action_uid.clone()) {
        entities_vec.push(cedar_policy::Entity::new(
            action_uid.clone(),
            Default::default(),
            Default::default(),
        )?);
    }
    let entities = Entities::from_entities(entities_vec, None)?;
    let request = Request::new(
        assembly.principal.clone(),
        action_uid,
        assembly.resource.clone(),
        Context::empty(),
        None,
    )?;
    Ok(Authorizer::new()
        .is_authorized(&request, &policies, &entities)
        .decision())
}

fn map_assembly_error(error: AssemblyError) -> LogicError {
    match error {
        AssemblyError::ResourceNotFound => LogicError::not_found("resource not found"),
        AssemblyError::Internal(message) => LogicError::internal(message),
    }
}
