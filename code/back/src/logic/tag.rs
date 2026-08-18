use crate::infrastructure::state::AppState;
use crate::logic::authorize::{authorize, authorize_or};
use crate::logic::error::{LogicError, database_error};
use crate::repository::authorization::Resource;
use crate::repository::role::{
    PERMISSION_TAG_APPLY, PERMISSION_TAG_CREATE, PERMISSION_TAG_DELETE, PERMISSION_TAG_READ,
    PERMISSION_TAG_UNAPPLY, PERMISSION_TAG_UPDATE,
};
use crate::repository::tag::{
    apply_tag_to_article, count_tag_articles, create_tag as create_tag_node,
    delete_tag as delete_tag_node, read_tag_articles, read_tag_by_id, read_tag_by_name,
    read_tags as read_tag_nodes, unapply_tag_from_article, update_tag as update_tag_node,
};
use nail_common::response::tag::{TagListItem, TagListPage, TagView};

fn tag_console() -> Resource {
    Resource::Virtual("any".to_string())
}

pub async fn create_tag(
    state: &AppState,
    actor_id: &str,
    raw_name: &str,
) -> Result<(String, String), LogicError> {
    authorize(state, actor_id, PERMISSION_TAG_CREATE, &tag_console()).await?;
    let name = nail_common::tag::validate_tag_name(raw_name)
        .map_err(|error| LogicError::bad_request(error.to_string()))?;
    if read_tag_by_name(&state.graph, &name)
        .await
        .map_err(database_error)?
        .is_some()
    {
        return Err(LogicError::bad_request("tag already exists"));
    }
    let tag_id = create_tag_node(&state.graph, &name)
        .await
        .map_err(|error| LogicError::internal(format!("failed to create tag: {error}")))?;
    Ok((tag_id, name))
}

pub async fn read_tags(
    state: &AppState,
    actor_id: &str,
    page: u64,
    limit: u64,
) -> Result<TagListPage, LogicError> {
    authorize(state, actor_id, PERMISSION_TAG_READ, &tag_console()).await?;
    let tags = read_tag_nodes(&state.graph).await.map_err(database_error)?;
    let total = tags.len() as u64;
    let offset = page.saturating_sub(1).saturating_mul(limit);
    let page_tags = &tags[usize::try_from(offset).unwrap_or(usize::MAX)
        ..usize::try_from(offset + limit)
            .unwrap_or(tags.len())
            .min(tags.len())];

    let mut tag_list = Vec::with_capacity(page_tags.len());
    for tag in page_tags {
        let article_count = count_tag_articles(&state.graph, &tag.id)
            .await
            .map_err(database_error)?;
        tag_list.push(TagListItem {
            id: tag.id.clone(),
            name: tag.tag_name.clone(),
            article_count,
        });
    }
    let has_next = page < total.div_ceil(limit);
    Ok(TagListPage {
        tag_list,
        has_next,
        total,
    })
}

pub async fn read_tag(
    state: &AppState,
    actor_id: &str,
    tag_id: &str,
) -> Result<TagView, LogicError> {
    authorize(state, actor_id, PERMISSION_TAG_READ, &tag_console()).await?;
    let tag = read_tag_by_id(&state.graph, tag_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::not_found("tag not found"))?;
    let article_count = count_tag_articles(&state.graph, &tag.id)
        .await
        .map_err(database_error)?;
    Ok(TagView {
        id: tag.id,
        name: tag.tag_name,
        article_count,
    })
}

pub async fn update_tag(
    state: &AppState,
    actor_id: &str,
    tag_id: &str,
    raw_name: &str,
) -> Result<(String, String), LogicError> {
    authorize(
        state,
        actor_id,
        PERMISSION_TAG_UPDATE,
        &Resource::Tag(tag_id.to_string()),
    )
    .await?;
    let name = nail_common::tag::validate_tag_name(raw_name)
        .map_err(|error| LogicError::bad_request(error.to_string()))?;
    if read_tag_by_id(&state.graph, tag_id)
        .await
        .map_err(database_error)?
        .is_none()
    {
        return Err(LogicError::not_found("tag not found"));
    }
    if let Some(existing) = read_tag_by_name(&state.graph, &name)
        .await
        .map_err(database_error)?
        && existing.id != tag_id
    {
        return Err(LogicError::bad_request("tag name already exists"));
    }
    update_tag_node(&state.graph, tag_id, &name)
        .await
        .map_err(|error| LogicError::internal(format!("failed to update tag: {error}")))?;
    Ok((tag_id.to_string(), name))
}

pub async fn delete_tag(state: &AppState, actor_id: &str, tag_id: &str) -> Result<(), LogicError> {
    authorize(
        state,
        actor_id,
        PERMISSION_TAG_DELETE,
        &Resource::Tag(tag_id.to_string()),
    )
    .await?;
    if read_tag_by_id(&state.graph, tag_id)
        .await
        .map_err(database_error)?
        .is_none()
    {
        return Err(LogicError::not_found("tag not found"));
    }
    delete_tag_node(&state.graph, tag_id)
        .await
        .map_err(|error| LogicError::internal(format!("failed to delete tag: {error}")))?;
    Ok(())
}

#[allow(dead_code)]
pub async fn read_tag_detail(
    state: &AppState,
    actor_id: &str,
    tag_id: &str,
) -> Result<(TagView, Vec<String>), LogicError> {
    authorize(state, actor_id, PERMISSION_TAG_READ, &tag_console()).await?;
    let tag = read_tag_by_id(&state.graph, tag_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| LogicError::not_found("tag not found"))?;
    let article_count = count_tag_articles(&state.graph, &tag.id)
        .await
        .map_err(database_error)?;
    let article_ids = read_tag_articles(&state.graph, &tag.id)
        .await
        .map_err(database_error)?;
    Ok((
        TagView {
            id: tag.id,
            name: tag.tag_name,
            article_count,
        },
        article_ids,
    ))
}

pub async fn apply_tag(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    tag_id: &str,
) -> Result<(), LogicError> {
    authorize_or(
        state,
        actor_id,
        PERMISSION_TAG_APPLY,
        &Resource::Tag(tag_id.to_string()),
        "tag not found",
    )
    .await?;
    if !crate::repository::article::article_exists(&state.graph, article_id)
        .await
        .map_err(database_error)?
    {
        return Err(LogicError::not_found("article not found"));
    }
    apply_tag_to_article(&state.graph, article_id, tag_id)
        .await
        .map_err(|error| LogicError::internal(format!("failed to apply tag: {error}")))?;
    Ok(())
}

pub async fn unapply_tag(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    tag_id: &str,
) -> Result<(), LogicError> {
    authorize_or(
        state,
        actor_id,
        PERMISSION_TAG_UNAPPLY,
        &Resource::Tag(tag_id.to_string()),
        "tag not found",
    )
    .await?;
    if !crate::repository::article::article_exists(&state.graph, article_id)
        .await
        .map_err(database_error)?
    {
        return Err(LogicError::not_found("article not found"));
    }
    unapply_tag_from_article(&state.graph, article_id, tag_id)
        .await
        .map_err(|error| LogicError::internal(format!("failed to unapply tag: {error}")))?;
    Ok(())
}
