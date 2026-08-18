use std::collections::HashSet;

use agdb::{DbError, QueryBuilder};
use nail_common::search::SearchRange;
use seekstorm::index::Document;

use crate::repository::graph::{DbHandle, read_node_sync, read_rows_sync, resolve_node_id_sync};
use crate::repository::schema::{
    ArticleRow, CommentRow, EDGE_ARTICLE_APPLY_TAG, EDGE_ARTICLE_HOLD_VERSION,
    EDGE_COMMENT_ATTACH_VERSION, EDGE_COMMENT_REPLY_COMMENT, EDGE_USER_AUTHOR_ARTICLE,
    EDGE_USER_AUTHOR_COMMENT, EDGE_USER_HOLD_ROLE, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_USER, KEY_TYPE,
    RoleRow, TagRow, UserRow, VersionRow,
};

use super::schema::{
    FIELD_ARTICLE_ID, FIELD_AUTHOR_ID, FIELD_AUTHOR_NAME, FIELD_COMMENT_ID, FIELD_CONTENT,
    FIELD_DOC_TYPE, FIELD_NOTE, FIELD_ROLE, FIELD_SUMMARY, FIELD_TAGS, FIELD_TITLE, FIELD_TS,
    FIELD_VERSION_ID, FIELD_VERSION_NUMBER,
};
use super::{SearchCommentOutcome, SearchHitOutcome, SearchVersionOutcome};

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

fn read_highlighted_or_raw(document: &Document, field: &str) -> String {
    let highlighted = read_string_field(document, &highlight_name(field));
    if highlighted.is_empty() {
        read_string_field(document, field)
    } else {
        highlighted
    }
}

fn field_hit(document: &Document, field: &str, query_terms: &[String]) -> bool {
    let raw = read_string_field(document, field);
    let folded = raw.to_lowercase();
    query_terms.iter().any(|term| folded.contains(term))
}

pub(super) fn read_version_outcome(
    document: &Document,
    effective_ranges: &[SearchRange],
    query_terms: &[String],
) -> SearchVersionOutcome {
    let mut article_hits = Vec::new();
    let mut version_hits = Vec::new();
    let mut version_number_hit = false;
    for range in effective_ranges {
        match range {
            SearchRange::Summary => {
                if field_hit(document, FIELD_SUMMARY, query_terms) {
                    article_hits.push(SearchHitOutcome {
                        range: SearchRange::Summary,
                        snippet: read_highlighted_or_raw(document, FIELD_SUMMARY),
                    });
                }
            }
            SearchRange::Tag => {
                if field_hit(document, FIELD_TAGS, query_terms) {
                    article_hits.push(SearchHitOutcome {
                        range: SearchRange::Tag,
                        snippet: read_highlighted_or_raw(document, FIELD_TAGS),
                    });
                }
            }
            SearchRange::Note => {
                if field_hit(document, FIELD_NOTE, query_terms) {
                    version_hits.push(SearchHitOutcome {
                        range: SearchRange::Note,
                        snippet: read_highlighted_or_raw(document, FIELD_NOTE),
                    });
                }
            }
            SearchRange::VersionNumber => {
                version_number_hit = field_hit(document, FIELD_VERSION_NUMBER, query_terms);
            }
            _ => {}
        }
    }
    SearchVersionOutcome {
        article_id: read_string_field(document, FIELD_ARTICLE_ID),
        version_id: read_string_field(document, FIELD_VERSION_ID),
        version_number: read_highlighted_or_raw(document, FIELD_VERSION_NUMBER),
        title: read_highlighted_or_raw(document, FIELD_TITLE),
        author_id: read_string_field(document, FIELD_AUTHOR_ID),
        author_name: read_highlighted_or_raw(document, FIELD_AUTHOR_NAME),
        article_hits,
        version_hits,
        version_number_hit,
    }
}

pub(super) fn read_comment_outcome(document: &Document) -> SearchCommentOutcome {
    SearchCommentOutcome {
        article_id: read_string_field(document, FIELD_ARTICLE_ID),
        version_id: read_string_field(document, FIELD_VERSION_ID),
        comment_id: read_string_field(document, FIELD_COMMENT_ID),
        author_id: read_string_field(document, FIELD_AUTHOR_ID),
        author_name: read_highlighted_or_raw(document, FIELD_AUTHOR_NAME),
        content: read_highlighted_or_raw(document, FIELD_CONTENT),
        article_title: String::new(),
        article_author_name: String::new(),
        version_number: String::new(),
    }
}

fn read_user_roles_sync(guard: &agdb::DbAny, user_id: &str) -> Result<String, DbError> {
    let Some(user_db_id) = resolve_node_id_sync(guard, ENTITY_TYPE_USER, user_id)? else {
        return Ok(String::new());
    };
    let edges = guard.exec(
        QueryBuilder::search()
            .from(user_db_id)
            .where_()
            .distance(agdb::CountComparison::Equal(1))
            .and()
            .edge()
            .and()
            .key(KEY_TYPE)
            .value(EDGE_USER_HOLD_ROLE)
            .query(),
    )?;
    let mut roles = Vec::new();
    for edge in &edges.elements {
        if let Some(row) = read_node_sync::<RoleRow>(guard, edge.to)? {
            roles.push(row.role_name);
        }
    }
    roles.sort();
    Ok(roles.join(","))
}

pub(super) async fn build_documents(
    db: &DbHandle,
    article_id: &str,
) -> anyhow::Result<Vec<Document>> {
    let guard = db.read().await;
    let Some(article) = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, article_id)? else {
        return Ok(Vec::new());
    };
    if crate::repository::delete::has_soft_deleted_flag(&guard, article)? {
        return Ok(Vec::new());
    }
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
    let (author_id, author_name) = read_owner(&guard, article, EDGE_USER_AUTHOR_ARTICLE)?;
    let author_role = if author_id.is_empty() {
        String::new()
    } else {
        read_user_roles_sync(&guard, &author_id)?
    };
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
            .value(EDGE_ARTICLE_HOLD_VERSION)
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
        let version_deleted =
            crate::repository::delete::has_soft_deleted_flag(&guard, version_edge.to)?;

        if version_deleted {
            continue;
        }
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
        version_doc.insert(FIELD_AUTHOR_ID.to_string(), serde_json::json!(author_id));
        version_doc.insert(FIELD_ROLE.to_string(), serde_json::json!(author_role));
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
            if crate::repository::delete::has_soft_deleted_flag(&guard, comment_node)? {
                continue;
            }
            let (comment_author_id, comment_author) =
                read_owner(&guard, comment_node, EDGE_USER_AUTHOR_COMMENT)?;
            let comment_author_role = if comment_author_id.is_empty() {
                String::new()
            } else {
                read_user_roles_sync(&guard, &comment_author_id)?
            };
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
                FIELD_AUTHOR_ID.to_string(),
                serde_json::json!(comment_author_id),
            );
            comment_doc.insert(
                FIELD_ROLE.to_string(),
                serde_json::json!(comment_author_role),
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

fn comments_of_version(
    guard: &agdb::DbAny,
    version: agdb::DbId,
) -> Result<Vec<agdb::DbId>, DbError> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    let mut stack: Vec<agdb::DbId> =
        incoming_comment_nodes(guard, version, EDGE_COMMENT_ATTACH_VERSION)?;
    while let Some(node) = stack.pop() {
        if !seen.insert(node) {
            continue;
        }
        result.push(node);
        let replies = incoming_comment_nodes(guard, node, EDGE_COMMENT_REPLY_COMMENT)?;
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

fn read_owner(
    guard: &agdb::DbAny,
    node: agdb::DbId,
    edge_type: &str,
) -> Result<(String, String), DbError> {
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
        Some(edge) => {
            let rows = read_rows_sync::<UserRow>(guard, &[edge.from])?;
            let row = rows.into_iter().next();
            (
                row.as_ref().map(|r| r.id.clone()).unwrap_or_default(),
                row.map(|r| r.name).unwrap_or_default(),
            )
        }
        None => (String::new(), String::new()),
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
            .value(EDGE_ARTICLE_APPLY_TAG)
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
