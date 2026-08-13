use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;
use crate::repository::role::{
    PERMISSION_ARTICLE_UPDATE, user_holds_permission,
};

pub async fn require_permission(
    state: &AppState,
    actor_id: &str,
    permission: &str,
) -> Result<(), LogicError> {
    let granted = user_holds_permission(&state.graph, actor_id, permission)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
    if granted {
        Ok(())
    } else {
        Err(LogicError::forbidden("you are denied"))
    }
}

pub async fn require_owner_or_permission_for_article(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    permission: &str,
) -> Result<(), LogicError> {
    let owner = crate::repository::article::owner_of(&state.graph, article_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
    let Some(owner) = owner else {
        return Err(LogicError::not_found("article not found"));
    };
    if owner == actor_id {
        return Ok(());
    }
    require_permission(state, actor_id, permission).await
}

pub async fn require_owner_or_permission_for_version(
    state: &AppState,
    actor_id: &str,
    version_id: &str,
    permission: &str,
) -> Result<(), LogicError> {
    let parent_article = crate::repository::version::parent_article_of(&state.graph, version_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
    let Some(parent_article) = parent_article else {
        return Err(LogicError::not_found("version not found"));
    };
    let owner = crate::repository::article::owner_of(&state.graph, &parent_article)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
    if owner.as_deref() == Some(actor_id) {
        return Ok(());
    }
    require_permission(state, actor_id, permission).await
}

pub async fn is_article_author(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
) -> Result<bool, LogicError> {
    let owner = crate::repository::article::owner_of(&state.graph, article_id)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))?;
    if owner.as_deref() == Some(actor_id) {
        return Ok(true);
    }
    user_holds_permission(&state.graph, actor_id, PERMISSION_ARTICLE_UPDATE)
        .await
        .map_err(|error| LogicError::internal(format!("database query failed: {error}")))
}
