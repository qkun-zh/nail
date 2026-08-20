use std::collections::HashSet;

use nail_common::request::DeleteMode;
use nail_common::response::comment::{CommentIdView, CommentView};
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::logic::authorize::{
    EntityRef, authorize_entity_or, authorize_global, require_entity_visible,
};
use crate::logic::error::{LogicError, MAX_COMMENT_TREE_DEPTH, database_error};
use crate::logic::pagination::page_offset;
use crate::logic::search::sync_article_best_effort;
use crate::repository::comment::{
    CommentTreeItem, CreateCommentError, create_reply_comment, create_top_level_comment,
    read_comment_children_page, read_comment_item, read_comments_page_by_version,
    update_comment_content, version_of_comment,
};
use crate::repository::role::{
    PERMISSION_COMMENT_CREATE, PERMISSION_COMMENT_DELETE_HARD, PERMISSION_COMMENT_DELETE_SOFT,
    PERMISSION_COMMENT_DELETE_TRANSFER, PERMISSION_COMMENT_READ, PERMISSION_COMMENT_UNDELETE_SOFT,
    PERMISSION_COMMENT_UPDATE,
};
use crate::repository::transfer::{TransferTargetError, transfer_comment};
use crate::repository::version::{parent_article_of, read_version};

pub async fn create_comment(
    state: &AppState,
    actor_id: &str,
    version_id: &str,
    raw_content: &str,
) -> Result<String, LogicError> {
    authorize_global(state, actor_id, PERMISSION_COMMENT_CREATE).await?;
    let content =
        validate_comment_content(raw_content, state.configurator.max_comment_body_chars())?;
    let comment_id = Uuid::now_v7().to_string();
    create_top_level_comment(&state.database, &comment_id, actor_id, version_id, &content).await?;
    sync_article_best_effort_for_version(state, version_id).await;
    Ok(comment_id)
}

pub async fn create_reply(
    state: &AppState,
    actor_id: &str,
    parent_comment_id: &str,
    raw_content: &str,
) -> Result<String, LogicError> {
    authorize_global(state, actor_id, PERMISSION_COMMENT_CREATE).await?;
    let content =
        validate_comment_content(raw_content, state.configurator.max_comment_body_chars())?;
    let comment_id = Uuid::now_v7().to_string();
    create_reply_comment(
        &state.database,
        &comment_id,
        actor_id,
        parent_comment_id,
        &content,
        MAX_COMMENT_TREE_DEPTH,
    )
    .await
    .map_err(|error| match error {
        CreateCommentError::TargetNotFound => LogicError::not_found(
            "reply target not found (the parent comment may have been removed)",
        ),
        other => other.into(),
    })?;
    sync_article_best_effort_for_comment(state, parent_comment_id).await;
    Ok(comment_id)
}

pub async fn read_comments(
    state: &AppState,
    actor_id: &str,
    version_id: &str,
    page: u64,
    limit: u64,
) -> Result<nail_common::response::ListPage<CommentView>, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_COMMENT_READ,
        EntityRef::Version(version_id),
    )
    .await?;
    if read_version(&state.database, version_id).await?.is_none() {
        return Err(LogicError::not_found("version not found"));
    }
    require_entity_visible(state, actor_id, EntityRef::Version(version_id)).await?;

    let total =
        crate::repository::comment::count_comments_by_version(&state.database, version_id).await?;
    let offset = page_offset(page, limit);
    let (items, has_next) =
        read_comments_page_by_version(&state.database, version_id, limit, offset).await?;

    let items = build_comment_views(state, items).await?;

    Ok(nail_common::response::ListPage {
        items,
        has_next,
        total,
    })
}

pub async fn read_comment(
    state: &AppState,
    actor_id: &str,
    comment_id: &str,
) -> Result<CommentView, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_COMMENT_READ,
        EntityRef::Comment(comment_id),
    )
    .await?;
    let item = read_comment_item(&state.database, comment_id)
        .await?
        .ok_or_else(|| LogicError::not_found("comment not found"))?;
    require_entity_visible(state, actor_id, EntityRef::Comment(comment_id)).await?;
    to_comment_view(state, item).await
}

pub async fn read_comment_children(
    state: &AppState,
    actor_id: &str,
    parent_comment_id: &str,
    page: u64,
    limit: u64,
) -> Result<nail_common::response::ListPage<CommentView>, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_COMMENT_READ,
        EntityRef::Comment(parent_comment_id),
    )
    .await?;
    let total =
        crate::repository::comment::count_comment_children(&state.database, parent_comment_id).await?;
    let offset = page_offset(page, limit);
    let (items, has_next) =
        read_comment_children_page(&state.database, parent_comment_id, limit, offset)
            .await
            .map_err(|error| {
                if crate::repository::graph::is_not_found(&error) {
                    LogicError::not_found("comment not found")
                } else {
                    database_error(error)
                }
            })?;
    let items = build_comment_views(state, items).await?;
    Ok(nail_common::response::ListPage {
        items,
        has_next,
        total,
    })
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
    let user_names = crate::repository::user::read_user_names(&state.database, &user_ids).await?;

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
        &state.database,
        std::slice::from_ref(&item.author_id),
    )
    .await?;
    to_comment_view_with_names(item, &user_names)
}

pub async fn update_comment(
    state: &AppState,
    actor_id: &str,
    comment_id: &str,
    raw_content: &str,
) -> Result<CommentIdView, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_COMMENT_UPDATE,
        EntityRef::Comment(comment_id),
    )
    .await?;
    let content =
        validate_comment_content(raw_content, state.configurator.max_comment_body_chars())?;
    let found = update_comment_content(&state.database, comment_id, &content).await?;
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
            authorize_entity_or(
                state,
                actor_id,
                PERMISSION_COMMENT_DELETE_TRANSFER,
                EntityRef::Comment(comment_id),
            )
            .await?;
            transfer_comment(&state.database, comment_id)
                .await
                .map_err(|error| match error {
                    TransferTargetError::TargetMissing => {
                        LogicError::not_found("comment not found")
                    }
                    TransferTargetError::TargetOwnerMissing => {
                        LogicError::internal("comment has no owner")
                    }
                    other => other.into(),
                })?;
        }
        Some(DeleteMode::Hard) => {
            authorize_entity_or(
                state,
                actor_id,
                PERMISSION_COMMENT_DELETE_HARD,
                EntityRef::Comment(comment_id),
            )
            .await?;
            crate::repository::delete::delete_comment(&state.database, comment_id).await?;
        }
        Some(DeleteMode::Soft) => {
            authorize_entity_or(
                state,
                actor_id,
                PERMISSION_COMMENT_DELETE_SOFT,
                EntityRef::Comment(comment_id),
            )
            .await?;
            let already_deleted =
                crate::repository::delete::is_soft_deleted(&state.database, "comment", comment_id)
                    .await?;
            if already_deleted {
                return Err(LogicError::bad_request("already soft-deleted"));
            }
            crate::repository::delete::soft_delete_comment(&state.database, comment_id).await?;
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

pub async fn undelete_soft_comment(
    state: &AppState,
    actor_id: &str,
    comment_id: &str,
) -> Result<CommentIdView, LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_COMMENT_UNDELETE_SOFT,
        EntityRef::Comment(comment_id),
    )
    .await?;
    let hidden =
        crate::repository::delete::is_soft_deleted(&state.database, "comment", comment_id).await?;
    if !hidden {
        return Err(LogicError::bad_request("not soft-deleted"));
    }
    crate::repository::delete::clear_soft_deleted_flag(&state.database, comment_id).await?;
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

async fn sync_article_best_effort_for_comment(state: &AppState, comment_id: &str) {
    let Some(version_id) = version_of_comment(&state.database, comment_id)
        .await
        .ok()
        .flatten()
    else {
        return;
    };
    sync_article_best_effort_for_version(state, &version_id).await;
}

async fn sync_article_best_effort_for_version(state: &AppState, version_id: &str) {
    let Some(article_id) = parent_article_of(&state.database, version_id)
        .await
        .ok()
        .flatten()
    else {
        return;
    };
    sync_article_best_effort(state, &article_id).await;
}
