use database::NodeKind;

use crate::infrastructure::state::AppState;
use crate::logic::authorize::{EntityRef, authorize_entity_or};
use crate::logic::error::LogicError;

pub(crate) fn soft_delete_guard(
    state: &AppState,
    actor_id: &str,
    entity: EntityRef<'_>,
    permission: &str,
    kind: NodeKind,
    id: &str,
) -> Result<(), LogicError> {
    authorize_entity_or(state, actor_id, permission, entity)?;
    let already_deleted = crate::repository::delete::is_soft_deleted(&state.database, kind, id)?;
    if already_deleted {
        return Err(LogicError::bad_request("already soft-deleted"));
    }
    Ok(())
}

pub(crate) fn undelete_guard(
    state: &AppState,
    actor_id: &str,
    entity: EntityRef<'_>,
    permission: &str,
    kind: NodeKind,
    id: &str,
) -> Result<(), LogicError> {
    authorize_entity_or(state, actor_id, permission, entity)?;
    let hidden = crate::repository::delete::is_soft_deleted(&state.database, kind, id)?;
    if !hidden {
        return Err(LogicError::bad_request("not soft-deleted"));
    }
    Ok(())
}
