use agdb::{DbError, QueryBuilder};
use seekstorm::index::Document;

use crate::repository::graph::{DbHandle, read_rows_sync, resolve_node_id_sync};
use crate::repository::schema::{
    ArticleRow, CommentRow, EDGE_ARTICLE_TO_TAG, EDGE_ARTICLE_TO_VERSION, EDGE_COMMENT_TO_VERSION,
    EDGE_USER_TO_ARTICLE, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_VERSION, KEY_TYPE, TagRow, UserRow,
    VersionRow,
};

use super::{
    FIELD_AUTHOR, FIELD_COMMENT, FIELD_ID, FIELD_NOTE, FIELD_SUMMARY, FIELD_TAG, FIELD_TITLE,
    FIELD_TS,
};

pub(super) fn read_string_field(document: &Document, field: &str) -> String {
    document
        .get(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

pub(super) fn read_i64_field(document: &Document, field: &str) -> i64 {
    document.get(field).and_then(|value| value.as_i64()).unwrap_or(0)
}

pub(super) async fn build_document(db: &DbHandle, article_id: &str) -> anyhow::Result<Option<Document>> {
    let guard = db.read().await;
    let Some(article) = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(None);
    };
    let article_row = read_rows_sync::<ArticleRow>(&guard, &[article])?
        .into_iter()
        .next();
    let title = article_row.as_ref().map(|row| row.title.clone()).unwrap_or_default();
    let summary = article_row
        .as_ref()
        .map(|row| row.summary.clone())
        .unwrap_or_default();
    let latest_version_id = article_row
        .as_ref()
        .and_then(|row| row.latest_version_id.clone())
        .unwrap_or_default();

    let author = read_owner_name(&guard, article)?;
    let note = read_latest_note(&guard, &latest_version_id)?;
    let ts = nail_common::time::uuidv7_timestamp_ms(&latest_version_id)
        .map(|millis| (millis / 1000) as i64)
        .unwrap_or(0);
    let tags = read_tag_names(&guard, article)?;
    let comments = read_comment_contents(&guard, article)?;

    let mut document = Document::new();
    document.insert(FIELD_ID.to_string(), serde_json::json!(article_id));
    document.insert(FIELD_TITLE.to_string(), serde_json::json!(title));
    document.insert(FIELD_SUMMARY.to_string(), serde_json::json!(summary));
    document.insert(FIELD_AUTHOR.to_string(), serde_json::json!(author));
    document.insert(FIELD_NOTE.to_string(), serde_json::json!(note));
    document.insert(FIELD_TAG.to_string(), serde_json::json!(tags));
    document.insert(FIELD_COMMENT.to_string(), serde_json::json!(comments));
    document.insert(FIELD_TS.to_string(), serde_json::json!(ts));
    Ok(Some(document))
}

fn read_owner_name(guard: &agdb::DbAny, article: agdb::DbId) -> Result<String, DbError> {
    let edges = guard.exec(
        QueryBuilder::search()
            .to(article)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_TO_ARTICLE)
            .query(),
    )?;
    Ok(match edges.elements.first() {
        Some(edge) => read_rows_sync::<UserRow>(guard, &[edge.from])?
            .into_iter()
            .next()
            .map(|row| row.name)
            .unwrap_or_default(),
        None => String::new(),
    })
}

fn read_latest_note(guard: &agdb::DbAny, latest_version_id: &str) -> Result<String, DbError> {
    if latest_version_id.is_empty() {
        return Ok(String::new());
    }
    let Some(version) = resolve_node_id_sync(guard, ENTITY_TYPE_VERSION, latest_version_id)? else {
        return Ok(String::new());
    };
    Ok(read_rows_sync::<VersionRow>(guard, &[version])?
        .into_iter()
        .next()
        .map(|row| row.note)
        .unwrap_or_default())
}

fn read_tag_names(guard: &agdb::DbAny, article: agdb::DbId) -> Result<Vec<String>, DbError> {
    let edges = guard.exec(
        QueryBuilder::search()
            .from(article)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_TO_TAG)
            .query(),
    )?;
    let mut tags = Vec::with_capacity(edges.elements.len());
    for edge in &edges.elements {
        if let Some(name) = read_rows_sync::<TagRow>(guard, &[edge.to])?
            .into_iter()
            .next()
            .map(|row| row.tag_name)
        {
            tags.push(name);
        }
    }
    Ok(tags)
}

fn read_comment_contents(guard: &agdb::DbAny, article: agdb::DbId) -> Result<Vec<String>, DbError> {
    let version_edges = guard.exec(
        QueryBuilder::search()
            .from(article)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_TO_VERSION)
            .query(),
    )?;
    let mut comments = Vec::new();
    for version_edge in &version_edges.elements {
        let comment_edges = guard.exec(
            QueryBuilder::search()
                .to(version_edge.to)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_COMMENT_TO_VERSION)
                .query(),
        )?;
        for comment_edge in &comment_edges.elements {
            if let Some(content) =
                read_rows_sync::<CommentRow>(guard, &[comment_edge.from])?
                    .into_iter()
                    .next()
                    .map(|row| row.content)
            {
                comments.push(content);
            }
        }
    }
    Ok(comments)
}
