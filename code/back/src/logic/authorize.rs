use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;
use crate::repository::authorization::{AssemblyError, Resource, assemble};

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

fn map_assembly_error(error: AssemblyError) -> LogicError {
    match error {
        AssemblyError::ResourceNotFound => LogicError::not_found("resource not found"),
        AssemblyError::Internal(message) => LogicError::internal(message),
    }
}
