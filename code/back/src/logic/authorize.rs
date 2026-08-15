use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;
use crate::repository::authorization::{AssemblyError, Resource, assemble, assemble_principal};

pub async fn authorize(
    state: &AppState,
    actor_id: &str,
    action: &str,
    resource: &Resource,
) -> Result<(), LogicError> {
    let assembly = assemble(&state.graph, actor_id, resource.clone())
        .await
        .map_err(map_assembly_error)?;
    let allowed = crate::infrastructure::cedar::decide(
        &assembly.principal,
        action,
        &assembly.resource,
        assembly.entities,
    )
    .map_err(|error| LogicError::internal(format!("authorization evaluation failed: {error}")))?;
    if allowed {
        Ok(())
    } else {
        Err(LogicError::forbidden("you are denied"))
    }
}

pub async fn authorize_or(
    state: &AppState,
    actor_id: &str,
    action: &str,
    resource: &Resource,
    not_found_message: &str,
) -> Result<(), LogicError> {
    match authorize(state, actor_id, action, resource).await {
        Ok(()) => Ok(()),
        Err(LogicError::NotFound(_)) => Err(LogicError::not_found(not_found_message)),
        Err(error) => Err(error),
    }
}

pub async fn authorize_create(
    state: &AppState,
    actor_id: &str,
    action: &str,
) -> Result<(), LogicError> {
    let (principal, mut entities) = assemble_principal(&state.graph, actor_id)
        .await
        .map_err(map_assembly_error)?;
    let resource_uid: cedar_policy::EntityUid = "Article::\"__create__\""
        .parse()
        .map_err(|error| LogicError::internal(format!("invalid create resource uid: {error}")))?;
    entities.push(cedar_policy::Entity::new_no_attrs(
        resource_uid.clone(),
        std::collections::HashSet::default(),
    ));
    let allowed = crate::infrastructure::cedar::decide(&principal, action, &resource_uid, entities)
        .map_err(|error| {
            LogicError::internal(format!("authorization evaluation failed: {error}"))
        })?;
    if allowed {
        Ok(())
    } else {
        Err(LogicError::forbidden("you are denied"))
    }
}

fn map_assembly_error(error: AssemblyError) -> LogicError {
    match error {
        AssemblyError::ResourceNotFound => LogicError::not_found("resource not found"),
        AssemblyError::Internal(message) => LogicError::internal(message),
    }
}
