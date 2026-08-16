use std::collections::HashSet;

use nail_common::request::DeleteMode;
use nail_common::response::comment::{CommentIdView, CommentListPage, CommentView};
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::logic::authorize::{authorize_create, authorize_or};
use crate::logic::error::{LogicError, database_error};
use crate::logic::search::sync_article_best_effort;
use crate::repository::authorization::Resource;
use crate::repository::comment::{
    CommentTreeItem, CreateCommentError, create_reply_comment, create_top_level_comment,
    read_comment_children_page, read_comment_item, read_comments_page_by_version,
    update_comment_content, version_of_comment,
};
use crate::repository::role::{
    PERMISSION_COMMENT_CREATE, PERMISSION_COMMENT_DELETE_HARD, PERMISSION_COMMENT_DELETE_SOFT,
    PERMISSION_COMMENT_DELETE_TRANSFER, PERMISSION_COMMENT_UPDATE,
};
use crate::repository::transfer::{TransferTargetError, transfer_comment};
use crate::repository::version::{parent_article_of, read_version};

const MAX_COMMENT_TREE_DEPTH: usize = 64;

pub async fn create_comment(
    state: &AppState,
    actor_id: &str,
    version_id: &str,
    raw_content: &str,
) -> Result<String, LogicError> {
    authorize_create(state, actor_id, PERMISSION_COMMENT_CREATE).await?;
    let content =
        validate_comment_content(raw_content, state.config.server.max_comment_body_chars)?;
    let comment_id = Uuid::now_v7().to_string();
    create_top_level_comment(&state.graph, &comment_id, actor_id, version_id, &content)
        .await
        .map_err(|error| map_create_comment_error(error, false))?;
    sync_article_best_effort_for_version(state, version_id).await;
    Ok(comment_id)
}

pub async fn create_reply(
    state: &AppState,
    actor_id: &str,
    parent_comment_id: &str,
    raw_content: &str,
) -> Result<String, LogicError> {
    authorize_create(state, actor_id, PERMISSION_COMMENT_CREATE).await?;
    let content =
        validate_comment_content(raw_content, state.config.server.max_comment_body_chars)?;
    let comment_id = Uuid::now_v7().to_string();
    create_reply_comment(
        &state.graph,
        &comment_id,
        actor_id,
        parent_comment_id,
        &content,
        MAX_COMMENT_TREE_DEPTH,
    )
    .await
    .map_err(|error| map_create_comment_error(error, true))?;
    sync_article_best_effort_for_comment(state, parent_comment_id).await;
    Ok(comment_id)
}

pub async fn read_comments(
    state: &AppState,
    version_id: &str,
    page: u64,
    limit: u64,
) -> Result<CommentListPage, LogicError> {
    if read_version(&state.graph, version_id)
        .await
        .map_err(database_error)?
        .is_none()
    {
        return Err(LogicError::not_found("version not found"));
    }

    let offset = page.saturating_sub(1).saturating_mul(limit);
    let (items, has_next) = read_comments_page_by_version(&state.graph, version_id, limit, offset)
        .await
        .map_err(database_error)?;

    let comments = build_comment_views(state, items).await?;

    Ok(CommentListPage { comments, has_next })
}

pub async fn read_comment(
    state: &AppState,
    actor_id: &str,
    comment_id: &str,
) -> Result<CommentView, LogicError> {
    let _ = actor_id;
    let item = read_comment_item(&state.graph, comment_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::not_found("comment not found"))?;
    to_comment_view(state, item).await
}

pub async fn read_comment_children(
    state: &AppState,
    actor_id: &str,
    parent_comment_id: &str,
    page: u64,
    limit: u64,
) -> Result<CommentListPage, LogicError> {
    let _ = actor_id;
    if read_comment_item(&state.graph, parent_comment_id)
        .await
        .map_err(database_error)?
        .is_none()
    {
        return Err(LogicError::not_found("comment not found"));
    }
    let offset = page.saturating_sub(1).saturating_mul(limit);
    let (items, has_next) =
        read_comment_children_page(&state.graph, parent_comment_id, limit, offset)
            .await
            .map_err(database_error)?;
    let comments = build_comment_views(state, items).await?;
    Ok(CommentListPage { comments, has_next })
}

async fn build_comment_views(
    state: &AppState,
    items: Vec<CommentTreeItem>,
) -> Result<Vec<CommentView>, LogicError> {
    let mut seen_users: HashSet<String> = HashSet::new();
    let mut user_ids: Vec<String> = Vec::new();
    for item in &items {
        if !item.author_id.is_empty() && seen_users.insert(item.author_id.clone()) {
            user_ids.push(item.author_id.clone());
        }
    }
    let user_names = crate::repository::user::read_user_names(&state.graph, &user_ids)
        .await
        .map_err(database_error)?;

    items
        .into_iter()
        .map(|item| to_comment_view_with_names(item, &user_names))
        .collect()
}

fn to_comment_view_with_names(
    item: CommentTreeItem,
    user_names: &std::collections::HashMap<String, String>,
) -> Result<CommentView, LogicError> {
    let created_at = nail_common::time::uuidv7_timestamp_secs(&item.id)
        .ok_or_else(|| LogicError::bad_request("invalid comment id"))?;
    let user_name = user_names.get(&item.author_id).cloned().unwrap_or_default();
    Ok(CommentView {
        id: item.id,
        content: item.content,
        user_id: item.author_id,
        parent_id: item.parent_id,
        created_at,
        user_name,
        child_count: item.child_count,
    })
}

async fn to_comment_view(
    state: &AppState,
    item: CommentTreeItem,
) -> Result<CommentView, LogicError> {
    let user_names = crate::repository::user::read_user_names(
        &state.graph,
        std::slice::from_ref(&item.author_id),
    )
    .await
    .map_err(database_error)?;
    to_comment_view_with_names(item, &user_names)
}

pub async fn update_comment(
    state: &AppState,
    actor_id: &str,
    comment_id: &str,
    raw_content: &str,
) -> Result<CommentIdView, LogicError> {
    authorize_or(
        state,
        actor_id,
        PERMISSION_COMMENT_UPDATE,
        &Resource::Comment(comment_id.to_string()),
        "comment not found",
    )
    .await?;
    let content =
        validate_comment_content(raw_content, state.config.server.max_comment_body_chars)?;
    let found = update_comment_content(&state.graph, comment_id, &content)
        .await
        .map_err(database_error)?;
    if !found {
        return Err(LogicError::not_found("comment not found"));
    }
    sync_article_best_effort_for_comment(state, comment_id).await;
    Ok(CommentIdView {
        comment_id: comment_id.to_string(),
    })
}

pub async fn delete_comment(
    state: &AppState,
    actor_id: &str,
    comment_id: &str,
    mode: Option<DeleteMode>,
) -> Result<CommentIdView, LogicError> {
    match mode {
        Some(DeleteMode::Transfer) => {
            authorize_or(
                state,
                actor_id,
                PERMISSION_COMMENT_DELETE_TRANSFER,
                &Resource::Comment(comment_id.to_string()),
                "comment not found",
            )
            .await?;
            transfer_comment(&state.graph, comment_id)
                .await
                .map_err(map_transfer_error)?;
        }
        Some(DeleteMode::Hard) => {
            authorize_or(
                state,
                actor_id,
                PERMISSION_COMMENT_DELETE_HARD,
                &Resource::Comment(comment_id.to_string()),
                "comment not found",
            )
            .await?;
            crate::repository::delete::delete_comment(&state.graph, comment_id)
                .await
                .map_err(database_error)?;
        }
        Some(DeleteMode::Soft) => {
            authorize_or(
                state,
                actor_id,
                PERMISSION_COMMENT_DELETE_SOFT,
                &Resource::Comment(comment_id.to_string()),
                "comment not found",
            )
            .await?;
            crate::repository::delete::soft_delete_comment(&state.graph, comment_id)
                .await
                .map_err(database_error)?;
        }
        None => {
            return Err(LogicError::bad_request(
                "missing or unsupported delete mode (expected \"transfer\", \"soft\", or \"hard\")",
            ));
        }
    }
    sync_article_best_effort_for_comment(state, comment_id).await;
    Ok(CommentIdView {
        comment_id: comment_id.to_string(),
    })
}

fn validate_comment_content(raw: &str, max_chars: u64) -> Result<String, LogicError> {
    nail_common::text::validate_ascii_text(
        raw,
        usize::try_from(max_chars).unwrap_or(usize::MAX),
        true,
    )
    .map_err(|error| LogicError::bad_request(error.to_string()))
}

fn map_create_comment_error(error: CreateCommentError, is_reply: bool) -> LogicError {
    match error {
        CreateCommentError::TargetNotFound if is_reply => LogicError::not_found(
            "reply target not found (the parent comment may have been removed)",
        ),
        CreateCommentError::TargetNotFound => {
            LogicError::not_found("comment target not found (the version may have been removed)")
        }
        CreateCommentError::CommentIdExists => LogicError::internal("comment id already exists"),
        CreateCommentError::CommentTreeTooDeep => LogicError::bad_request(format!(
            "comment thread too deep (max {MAX_COMMENT_TREE_DEPTH} reply layers)"
        )),
        CreateCommentError::Db(error) => database_error(error),
    }
}

fn map_transfer_error(error: TransferTargetError) -> LogicError {
    match error {
        TransferTargetError::TargetMissing => LogicError::not_found("comment not found"),
        TransferTargetError::TargetOwnerMissing => LogicError::internal("comment has no owner"),
        TransferTargetError::NoRecycler => LogicError::internal("no recycler available"),
        TransferTargetError::Db(error) => database_error(error),
    }
}

async fn sync_article_best_effort_for_comment(state: &AppState, comment_id: &str) {
    let Some(version_id) = version_of_comment(&state.graph, comment_id)
        .await
        .ok()
        .flatten()
    else {
        return;
    };
    sync_article_best_effort_for_version(state, &version_id).await;
}

async fn sync_article_best_effort_for_version(state: &AppState, version_id: &str) {
    let Some(article_id) = parent_article_of(&state.graph, version_id)
        .await
        .ok()
        .flatten()
    else {
        return;
    };
    sync_article_best_effort(state, &article_id).await;
}
