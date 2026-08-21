use std::collections::HashSet;

use database::{Database, EdgeKind, Error, NodeKind};
use searcher::{CommentDoc, SearchDoc, VersionDoc};

use crate::repository::access::GraphRead;
use crate::repository::delete::has_soft_deleted_flag;
use crate::repository::schema::{ArticleRow, CommentRow, RoleRow, TagRow, UserRow, VersionRow};

pub(super) fn build_documents(db: &Database, article_id: &str) -> anyhow::Result<Vec<SearchDoc>> {
    let mut documents = Vec::new();
    build_documents_inner(db, article_id, &mut documents)?;
    Ok(documents)
}

fn build_documents_inner(
    db: &Database,
    article_id: &str,
    documents: &mut Vec<SearchDoc>,
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

        documents.push(SearchDoc::Version(VersionDoc {
            version_id: version_id.clone(),
            article_id: article_id.to_string(),
            version_number: version_row.version_number.clone(),
            title: title.clone(),
            summary: summary.clone(),
            author_name: author_name.clone(),
            author_id: author_id.clone(),
            role: author_role.clone(),
            note: version_row.note.clone(),
            tags: tags.clone(),
            ts: version_ts,
        }));

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

            documents.push(SearchDoc::Comment(CommentDoc {
                comment_id,
                version_id: version_id.clone(),
                article_id: article_id.to_string(),
                author_name: comment_author,
                author_id: comment_author_id,
                role: comment_author_role,
                content: comment_row.content,
                ts: comment_ts,
            }));
        }
    }
    Ok(())
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
