use std::collections::HashSet;

use common::search::SearchRange;
use database::{Database, EdgeKind, Error, NodeKind};
use seekstorm::index::Document;

use crate::repository::access::GraphRead;
use crate::repository::delete::has_soft_deleted_flag;
use crate::repository::schema::{ArticleRow, CommentRow, RoleRow, TagRow, UserRow, VersionRow};

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
        article_author_id: String::new(),
        article_author_name: String::new(),
        version_number: String::new(),
    }
}

fn read_user_roles(db: &Database, user_id: &str) -> Result<String, Error> {
    db.read(|scope| {
        let Some(user_db_id) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(String::new());
        };
        let held = scope.outgoing(user_db_id, EdgeKind::UserHoldRole)?;
        let rows = scope.scope_read_nodes::<RoleRow>(&held)?;
        let mut roles: Vec<String> = rows.into_iter().map(|row| row.role_name).collect();
        roles.sort();
        Ok(roles.join(","))
    })
}

pub(super) fn build_documents(db: &Database, article_id: &str) -> anyhow::Result<Vec<Document>> {
    let mut documents = Vec::new();
    build_documents_inner(db, article_id, &mut documents)?;
    Ok(documents)
}

fn build_documents_inner(
    db: &Database,
    article_id: &str,
    documents: &mut Vec<Document>,
) -> anyhow::Result<()> {
    let context = db.read(|scope| {
        let Some(article) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(None);
        };
        if has_soft_deleted_flag(scope, article)? {
            return Ok(None);
        }
        let article_row = scope.scope_read_node::<ArticleRow>(article)?;
        let title = article_row
            .as_ref()
            .map(|row| row.title.clone())
            .unwrap_or_default();
        let summary = article_row
            .as_ref()
            .map(|row| row.summary.clone())
            .unwrap_or_default();
        let (author_id, author_name) = read_owner(scope, article, EdgeKind::UserAuthorArticle)?;
        let tags = read_tag_names(scope, article)?;
        let versions = scope.outgoing(article, EdgeKind::ArticleHoldVersion)?;
        let version_rows = scope.scope_read_nodes::<VersionRow>(&versions)?;
        Ok(Some((
            title,
            summary,
            author_id,
            author_name,
            tags,
            versions,
            version_rows,
        )))
    })?;
    let Some((title, summary, author_id, author_name, tags, versions, version_rows)) = context
    else {
        return Ok(());
    };
    let author_role = if author_id.is_empty() {
        String::new()
    } else {
        read_user_roles(db, &author_id)?
    };

    for (version_node, version_row) in versions.into_iter().zip(version_rows) {
        let version_id = version_row.id;
        let version_ts = common::time::uuidv7_timestamp_secs(&version_id)
            .map_or(0, |secs| i64::try_from(secs).unwrap_or(0));
        if db.read(|scope| has_soft_deleted_flag(scope, version_node))? {
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

        for comment_node in comments_of_version(db, version_node)? {
            let Some(comment_row) =
                db.read(|scope| scope.scope_read_node::<CommentRow>(comment_node))?
            else {
                continue;
            };
            let comment_id = comment_row.id;
            if db.read(|scope| has_soft_deleted_flag(scope, comment_node))? {
                continue;
            }
            let (comment_author_id, comment_author) =
                read_owner_of(db, comment_node, EdgeKind::UserAuthorComment)?;
            let comment_author_role = if comment_author_id.is_empty() {
                String::new()
            } else {
                read_user_roles(db, &comment_author_id)?
            };
            let comment_ts = common::time::uuidv7_timestamp_secs(&comment_id)
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
    Ok(())
}

fn comments_of_version(
    db: &Database,
    version: database::NodeId,
) -> Result<Vec<database::NodeId>, Error> {
    db.read(|scope| {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        let mut stack: Vec<database::NodeId> =
            scope.incoming(version, EdgeKind::CommentAttachVersion)?;
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            result.push(node);
            let replies = scope.incoming(node, EdgeKind::CommentReplyComment)?;
            stack.extend(replies);
        }
        Ok(result)
    })
}

fn read_owner(
    scope: &(impl GraphRead + ?Sized),
    node: database::NodeId,
    edge_kind: EdgeKind,
) -> Result<(String, String), Error> {
    Ok(match scope.scope_incoming(node, edge_kind)?.first() {
        Some(user) => {
            let row = scope.scope_read_node::<UserRow>(*user)?;
            (
                row.as_ref().map(|r| r.id.clone()).unwrap_or_default(),
                row.map(|r| r.name).unwrap_or_default(),
            )
        }
        None => (String::new(), String::new()),
    })
}

fn read_owner_of(
    db: &Database,
    node: database::NodeId,
    edge_kind: EdgeKind,
) -> Result<(String, String), Error> {
    db.read(|scope| read_owner(scope, node, edge_kind))
}

fn read_tag_names(
    scope: &(impl GraphRead + ?Sized),
    article: database::NodeId,
) -> Result<Vec<String>, Error> {
    let edges = scope.scope_outgoing(article, EdgeKind::ArticleApplyTag)?;
    let rows = scope.scope_read_nodes::<TagRow>(&edges)?;
    Ok(rows.into_iter().map(|row| row.tag_name).collect())
}
