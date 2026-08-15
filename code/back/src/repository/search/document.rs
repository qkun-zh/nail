use std::collections::HashSet;

use agdb::{DbError, QueryBuilder};
use nail_common::search::SearchRange;
use seekstorm::index::Document;

use crate::repository::graph::{DbHandle, read_rows_sync, resolve_node_id_sync};
use crate::repository::schema::{
    ArticleRow, CommentRow, EDGE_ARTICLE_TO_TAG, EDGE_ARTICLE_TO_VERSION, EDGE_COMMENT_TO_COMMENT,
    EDGE_COMMENT_TO_VERSION, EDGE_USER_TO_ARTICLE, EDGE_USER_TO_COMMENT, ENTITY_TYPE_ARTICLE,
    KEY_TYPE, TagRow, UserRow, VersionRow,
};

use super::{
    FIELD_ARTICLE_ID, FIELD_AUTHOR_NAME, FIELD_COMMENT_ID, FIELD_CONTENT, FIELD_DOC_TYPE,
    FIELD_NOTE, FIELD_SUMMARY, FIELD_TAGS, FIELD_TITLE, FIELD_TS, FIELD_VERSION_ID,
    FIELD_VERSION_NUMBER, SearchCommentOutcome, SearchHitOutcome, SearchVersionOutcome,
};

pub(super) fn read_string_field(document: &Document, field: &str) -> String {
    document
        .get(field)
        .map(|value| match value {
            serde_json::Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

fn highlight_name(field: &str) -> String {
    format!("{field}_highlight")
}

/// The highlighted rendering of a field when the highlighter was enabled for it,
/// otherwise the raw stored value. Used for always-shown fields that may carry
/// `<mark>`.
fn read_highlighted_or_raw(document: &Document, field: &str) -> String {
    let highlighted = read_string_field(document, &highlight_name(field));
    if highlighted.is_empty() {
        read_string_field(document, field)
    } else {
        highlighted
    }
}

/// True when an enabled range actually matched this field: the highlighted
/// rendering differs from the raw stored value (no literal `<mark>` scanning).
fn field_hit(document: &Document, field: &str) -> bool {
    let highlighted = read_string_field(document, &highlight_name(field));
    let raw = read_string_field(document, field);
    !highlighted.is_empty() && highlighted != raw
}

pub(super) fn read_version_outcome(
    document: &Document,
    effective_ranges: &[SearchRange],
) -> SearchVersionOutcome {
    let mut article_hits = Vec::new();
    let mut version_hits = Vec::new();
    let mut version_number_hit = false;
    for range in effective_ranges {
        match range {
            SearchRange::Summary => {
                if field_hit(document, FIELD_SUMMARY) {
                    article_hits.push(SearchHitOutcome {
                        range: SearchRange::Summary,
                        snippet: read_highlighted_or_raw(document, FIELD_SUMMARY),
                    });
                }
            }
            SearchRange::Tag => {
                if field_hit(document, FIELD_TAGS) {
                    article_hits.push(SearchHitOutcome {
                        range: SearchRange::Tag,
                        snippet: read_highlighted_or_raw(document, FIELD_TAGS),
                    });
                }
            }
            SearchRange::Note => {
                if field_hit(document, FIELD_NOTE) {
                    version_hits.push(SearchHitOutcome {
                        range: SearchRange::Note,
                        snippet: read_highlighted_or_raw(document, FIELD_NOTE),
                    });
                }
            }
            SearchRange::VersionNumber => {
                version_number_hit = field_hit(document, FIELD_VERSION_NUMBER);
            }
            _ => {}
        }
    }
    SearchVersionOutcome {
        article_id: read_string_field(document, FIELD_ARTICLE_ID),
        version_id: read_string_field(document, FIELD_VERSION_ID),
        version_number: read_highlighted_or_raw(document, FIELD_VERSION_NUMBER),
        title: read_highlighted_or_raw(document, FIELD_TITLE),
        author_name: read_highlighted_or_raw(document, FIELD_AUTHOR_NAME),
        article_hits,
        version_hits,
        version_number_hit,
    }
}

pub(super) fn read_comment_outcome(
    document: &Document,
    effective_ranges: &[SearchRange],
) -> SearchCommentOutcome {
    let comment = SearchCommentOutcome {
        article_id: read_string_field(document, FIELD_ARTICLE_ID),
        version_id: read_string_field(document, FIELD_VERSION_ID),
        comment_id: read_string_field(document, FIELD_COMMENT_ID),
        author_name: read_highlighted_or_raw(document, FIELD_AUTHOR_NAME),
        content: read_highlighted_or_raw(document, FIELD_CONTENT),
        article_title: String::new(),
        article_author_name: String::new(),
        version_number: String::new(),
    };
    let _ = effective_ranges;
    comment
}

/// Build one document per version plus one per comment for an article.
pub(super) async fn build_documents(
    db: &DbHandle,
    article_id: &str,
) -> anyhow::Result<Vec<Document>> {
    let guard = db.read().await;
    let Some(article) = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(Vec::new());
    };
    let article_row = read_rows_sync::<ArticleRow>(&guard, &[article])?
        .into_iter()
        .next();
    let title = article_row
        .as_ref()
        .map(|row| row.title.clone())
        .unwrap_or_default();
    let summary = article_row
        .as_ref()
        .map(|row| row.summary.clone())
        .unwrap_or_default();
    let author_name = read_owner_name(&guard, article, EDGE_USER_TO_ARTICLE)?;
    let tags = read_tag_names(&guard, article)?;

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

    let mut documents = Vec::new();
    for version_edge in &version_edges.elements {
        let Some(version_row) = read_rows_sync::<VersionRow>(&guard, &[version_edge.to])?
            .into_iter()
            .next()
        else {
            continue;
        };
        let version_id = version_row.id;
        let version_ts = nail_common::time::uuidv7_timestamp_secs(&version_id)
            .map_or(0, |secs| i64::try_from(secs).unwrap_or(0));

        let mut version_doc = Document::new();
        version_doc.insert(
            FIELD_DOC_TYPE.to_string(),
            serde_json::json!(vec!["version"]),
        );
        version_doc.insert(FIELD_VERSION_ID.to_string(), serde_json::json!(version_id));
        version_doc.insert(FIELD_ARTICLE_ID.to_string(), serde_json::json!(article_id));
        version_doc.insert(
            FIELD_VERSION_NUMBER.to_string(),
            serde_json::json!(version_row.version_number),
        );
        version_doc.insert(FIELD_TITLE.to_string(), serde_json::json!(title));
        version_doc.insert(FIELD_SUMMARY.to_string(), serde_json::json!(summary));
        version_doc.insert(
            FIELD_AUTHOR_NAME.to_string(),
            serde_json::json!(author_name),
        );
        version_doc.insert(FIELD_NOTE.to_string(), serde_json::json!(version_row.note));
        version_doc.insert(FIELD_TAGS.to_string(), serde_json::json!(tags));
        version_doc.insert(FIELD_TS.to_string(), serde_json::json!(version_ts));
        documents.push(version_doc);

        for comment_node in comments_of_version(&guard, version_edge.to)? {
            let Some(comment_row) = read_rows_sync::<CommentRow>(&guard, &[comment_node])?
                .into_iter()
                .next()
            else {
                continue;
            };
            let comment_id = comment_row.id;
            let comment_author = read_owner_name(&guard, comment_node, EDGE_USER_TO_COMMENT)?;
            let comment_ts = nail_common::time::uuidv7_timestamp_secs(&comment_id)
                .map_or(0, |secs| i64::try_from(secs).unwrap_or(0));

            let mut comment_doc = Document::new();
            comment_doc.insert(
                FIELD_DOC_TYPE.to_string(),
                serde_json::json!(vec!["comment"]),
            );
            comment_doc.insert(FIELD_COMMENT_ID.to_string(), serde_json::json!(comment_id));
            comment_doc.insert(FIELD_VERSION_ID.to_string(), serde_json::json!(version_id));
            comment_doc.insert(FIELD_ARTICLE_ID.to_string(), serde_json::json!(article_id));
            comment_doc.insert(
                FIELD_AUTHOR_NAME.to_string(),
                serde_json::json!(comment_author),
            );
            comment_doc.insert(
                FIELD_CONTENT.to_string(),
                serde_json::json!(comment_row.content),
            );
            comment_doc.insert(FIELD_TS.to_string(), serde_json::json!(comment_ts));
            documents.push(comment_doc);
        }
    }
    Ok(documents)
}

/// Collect every comment node hanging off a version, including nested replies.
fn comments_of_version(
    guard: &agdb::DbAny,
    version: agdb::DbId,
) -> Result<Vec<agdb::DbId>, DbError> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    let mut stack: Vec<agdb::DbId> =
        incoming_comment_nodes(guard, version, EDGE_COMMENT_TO_VERSION)?;
    while let Some(node) = stack.pop() {
        if !seen.insert(node) {
            continue;
        }
        result.push(node);
        let replies = incoming_comment_nodes(guard, node, EDGE_COMMENT_TO_COMMENT)?;
        stack.extend(replies);
    }
    Ok(result)
}

fn incoming_comment_nodes(
    guard: &agdb::DbAny,
    node: agdb::DbId,
    edge_type: &str,
) -> Result<Vec<agdb::DbId>, DbError> {
    let edges = guard.exec(
        QueryBuilder::search()
            .to(node)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(edge_type)
            .query(),
    )?;
    Ok(edges.elements.iter().map(|edge| edge.from).collect())
}

fn read_owner_name(
    guard: &agdb::DbAny,
    node: agdb::DbId,
    edge_type: &str,
) -> Result<String, DbError> {
    let edges = guard.exec(
        QueryBuilder::search()
            .to(node)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(edge_type)
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
