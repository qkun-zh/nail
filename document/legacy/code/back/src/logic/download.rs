
use uuid::Uuid;

use crate::authorization::entity_store::Resource;
use crate::authorization::gate::{PERMISSION_ARTICLE_READ, authorize_or};
use crate::logic::authenticate::authenticate_session;
use crate::logic::error::LogicError;
use crate::other::AppState;
use crate::repo;

pub async fn handle_mint_download_url(
    state: &AppState,
    session_token: &str,
    article_id: &str,
    version_id: &str,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;

    authorize_or(
        state,
        &user_id,
        PERMISSION_ARTICLE_READ,
        &Resource::Article(article_id.to_string()),
        "article version not found",
    )
    .await?;

    crate::logic::version::get_public_pdf_path(state, article_id, version_id).await?;

    let token = Uuid::now_v7().to_string();
    repo::token::download::create_download_token(&state.cache, &token, version_id, &user_id);

    Ok(format!("/api/article/download?token={token}"))
}

pub async fn handle_consume_download(
    state: &AppState,
    session_token: &str,
    token: &str,
) -> Result<String, LogicError> {
    let user_id = authenticate_session(state, session_token)?;

    let entry = repo::token::download::find_download_token(&state.cache, token)
        .ok_or_else(|| LogicError::bad_request("invalid or expired download token"))?;
    if entry.user_id != user_id {
        return Err(LogicError::bad_request(
            "download token is bound to another account",
        ));
    }

    let article_id = repo::article::find_article_id_by_version(&state.db, &entry.version_id)
        .await
        .map_err(|e| LogicError::internal(format!("database query failed: {e}")))?
        .ok_or_else(|| LogicError::not_found("article version not found"))?;
    authorize_or(
        state,
        &user_id,
        PERMISSION_ARTICLE_READ,
        &Resource::Article(article_id.clone()),
        "article version not found",
    )
    .await?;
    let full_path =
        crate::logic::version::get_public_pdf_path(state, &article_id, &entry.version_id).await?;

    if repo::token::download::consume_download_token(&state.cache, token).is_none() {
        return Err(LogicError::bad_request("invalid or expired download token"));
    }

    Ok(full_path)
}
