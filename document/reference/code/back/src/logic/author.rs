
use crate::authorization::entity_store::Resource;
use crate::authorization::gate::{
    PERMISSION_ARTICLE_UPDATE, PERMISSION_COMMENT_DELETE, is_allowed,
};
use crate::logic::authenticate::authenticate_session;
use crate::logic::error::LogicError;
use crate::other::AppState;

pub async fn handle_is_author(
    state: &AppState,
    session_token: &str,
    article_id: Option<&str>,
    version_id: Option<&str>,
    comment_id: Option<&str>,
) -> Result<bool, LogicError> {
    let user_id = authenticate_session(state, session_token)?;

    let allowed = match (article_id, version_id, comment_id) {
        (Some(aid), None, None) => {
            is_allowed(
                state,
                &user_id,
                PERMISSION_ARTICLE_UPDATE,
                &Resource::Article(aid.to_string()),
            )
            .await
        }
        (None, Some(vid), None) => {
            is_allowed(
                state,
                &user_id,
                PERMISSION_ARTICLE_UPDATE,
                &Resource::Version(vid.to_string()),
            )
            .await
        }
        (None, None, Some(cid)) => {
            is_allowed(
                state,
                &user_id,
                PERMISSION_COMMENT_DELETE,
                &Resource::Comment(cid.to_string()),
            )
            .await
        }
        _ => {
            return Err(LogicError::bad_request(
                "exactly one of article_id, version_id or comment_id is required",
            ));
        }
    };

    Ok(allowed)
}
