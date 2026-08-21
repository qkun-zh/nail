use crate::infrastructure::state::AppState;
use crate::logic::authorize::{EntityRef, authorize_entity, authorize_entity_or, authorize_global};
use crate::logic::error::LogicError;
use crate::logic::pagination::paginate;
use crate::repository::role::{
    PERMISSION_TAG_APPLY, PERMISSION_TAG_CREATE, PERMISSION_TAG_DELETE, PERMISSION_TAG_READ,
    PERMISSION_TAG_UNAPPLY, PERMISSION_TAG_UPDATE,
};
use crate::repository::tag::{
    apply_tag_to_article, count_tag_articles, create_tag as create_tag_node,
    delete_tag as delete_tag_node, read_tag_by_id, read_tag_by_name, read_tags as read_tag_nodes,
    unapply_tag_from_article, update_tag as update_tag_node,
};
use common::response::tag::TagListItem;

pub fn create_tag(
    state: &AppState,
    actor_id: &str,
    raw_name: &str,
) -> Result<(String, String), LogicError> {
    authorize_global(state, actor_id, PERMISSION_TAG_CREATE)?;
    let name = common::tag::validate_tag_name(raw_name)
        .map_err(|error| LogicError::bad_request(error.to_string()))?;
    if read_tag_by_name(&state.database, &name)?.is_some() {
        return Err(LogicError::bad_request("tag already exists"));
    }
    let tag_id = create_tag_node(&state.database, &name)
        .map_err(|error| LogicError::internal(format!("failed to create tag: {error}")))?;
    Ok((tag_id, name))
}

pub fn read_tags(
    state: &AppState,
    actor_id: &str,
    page: u64,
    limit: u64,
) -> Result<common::response::ListPage<TagListItem>, LogicError> {
    authorize_global(state, actor_id, PERMISSION_TAG_READ)?;
    let tags = read_tag_nodes(&state.database)?;
    let total = tags.len() as u64;
    let (page_tags, has_next) = paginate(tags, page, limit);

    let mut items = Vec::with_capacity(page_tags.len());
    for tag in &page_tags {
        let article_count = count_tag_articles(&state.database, &tag.id)?;
        items.push(TagListItem {
            id: tag.id.clone(),
            name: tag.tag_name.clone(),
            article_count,
        });
    }
    Ok(common::response::ListPage {
        items,
        has_next,
        total,
    })
}

pub fn read_tag(state: &AppState, actor_id: &str, tag_id: &str) -> Result<TagListItem, LogicError> {
    authorize_global(state, actor_id, PERMISSION_TAG_READ)?;
    let tag = read_tag_by_id(&state.database, tag_id)?
        .ok_or_else(|| LogicError::not_found("tag not found"))?;
    let article_count = count_tag_articles(&state.database, &tag.id)?;
    Ok(TagListItem {
        id: tag.id,
        name: tag.tag_name,
        article_count,
    })
}

pub fn update_tag(
    state: &AppState,
    actor_id: &str,
    tag_id: &str,
    raw_name: &str,
) -> Result<(String, String), LogicError> {
    authorize_entity(
        state,
        actor_id,
        PERMISSION_TAG_UPDATE,
        EntityRef::Tag(tag_id),
    )?;
    let name = common::tag::validate_tag_name(raw_name)
        .map_err(|error| LogicError::bad_request(error.to_string()))?;
    if read_tag_by_id(&state.database, tag_id)?.is_none() {
        return Err(LogicError::not_found("tag not found"));
    }
    if let Some(existing) = read_tag_by_name(&state.database, &name)?
        && existing.id != tag_id
    {
        return Err(LogicError::bad_request("tag name already exists"));
    }
    update_tag_node(&state.database, tag_id, &name)
        .map_err(|error| LogicError::internal(format!("failed to update tag: {error}")))?;
    Ok((tag_id.to_string(), name))
}

pub fn delete_tag(state: &AppState, actor_id: &str, tag_id: &str) -> Result<(), LogicError> {
    authorize_entity(
        state,
        actor_id,
        PERMISSION_TAG_DELETE,
        EntityRef::Tag(tag_id),
    )?;
    if read_tag_by_id(&state.database, tag_id)?.is_none() {
        return Err(LogicError::not_found("tag not found"));
    }
    delete_tag_node(&state.database, tag_id)
        .map_err(|error| LogicError::internal(format!("failed to delete tag: {error}")))?;
    Ok(())
}

pub fn apply_tag(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    tag_id: &str,
) -> Result<(), LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_TAG_APPLY,
        EntityRef::Tag(tag_id),
    )?;
    if !crate::repository::article::article_exists(&state.database, article_id)? {
        return Err(LogicError::not_found("article not found"));
    }
    apply_tag_to_article(&state.database, article_id, tag_id)
        .map_err(|error| LogicError::internal(format!("failed to apply tag: {error}")))?;
    Ok(())
}

pub fn unapply_tag(
    state: &AppState,
    actor_id: &str,
    article_id: &str,
    tag_id: &str,
) -> Result<(), LogicError> {
    authorize_entity_or(
        state,
        actor_id,
        PERMISSION_TAG_UNAPPLY,
        EntityRef::Tag(tag_id),
    )?;
    if !crate::repository::article::article_exists(&state.database, article_id)? {
        return Err(LogicError::not_found("article not found"));
    }
    unapply_tag_from_article(&state.database, article_id, tag_id)
        .map_err(|error| LogicError::internal(format!("failed to unapply tag: {error}")))?;
    Ok(())
}
