use std::collections::{HashMap, HashSet};

use database::{Database, EdgeKind, Error, NodeKind};

use crate::repository::access::GraphRead;
use crate::repository::delete::has_soft_deleted_flag;
use crate::repository::schema::{ArticleRow, IdRow, UserRow, VersionRow};

use super::SearchCommentOutcome;

pub(super) fn enrich_comment_headers(
    db: &Database,
    comments: &mut [SearchCommentOutcome],
) -> anyhow::Result<()> {
    if comments.is_empty() {
        return Ok(());
    }

    let article_ids: HashSet<String> = comments.iter().map(|c| c.article_id.clone()).collect();
    let version_ids: HashSet<String> = comments.iter().map(|c| c.version_id.clone()).collect();

    let mut article_by_id: HashMap<String, database::NodeId> = HashMap::new();
    let mut title_by_node: HashMap<database::NodeId, String> = HashMap::new();
    for id in &article_ids {
        db.read(|scope| {
            if let Some(node) = scope.resolve(NodeKind::Article, id)?
                && let Some(row) = scope.scope_read_node::<ArticleRow>(node)?
            {
                title_by_node.insert(node, row.title);
                article_by_id.insert(id.clone(), node);
            }
            Ok(())
        })?;
    }

    let mut version_by_id: HashMap<String, database::NodeId> = HashMap::new();
    let mut version_number_by_node: HashMap<database::NodeId, String> = HashMap::new();
    for id in &version_ids {
        db.read(|scope| {
            if let Some(node) = scope.resolve(NodeKind::Version, id)?
                && let Some(row) = scope.scope_read_node::<VersionRow>(node)?
            {
                version_number_by_node.insert(node, row.version_number);
                version_by_id.insert(id.clone(), node);
            }
            Ok(())
        })?;
    }

    let mut author_by_article: HashMap<database::NodeId, database::NodeId> = HashMap::new();
    let mut author_by_node: HashMap<database::NodeId, (String, String)> = HashMap::new();
    for article_node in article_by_id.values().copied() {
        db.read(|scope| {
            if let Some(owner) = scope
                .incoming(article_node, EdgeKind::UserAuthorArticle)?
                .first()
                && let Some(row) = scope.scope_read_node::<UserRow>(*owner)?
            {
                author_by_article.insert(article_node, *owner);
                author_by_node.insert(*owner, (row.id, row.name));
            }
            Ok(())
        })?;
    }

    for comment in comments.iter_mut() {
        let article_node = article_by_id.get(comment.article_id.as_str());
        let author = article_node
            .and_then(|node| author_by_article.get(node))
            .and_then(|user_node| author_by_node.get(user_node));
        comment.article_title = article_node
            .and_then(|node| title_by_node.get(node))
            .cloned()
            .unwrap_or_default();
        comment.article_author_id = author.map(|(id, _)| id.clone()).unwrap_or_default();
        comment.article_author_name = author.map(|(_, name)| name.clone()).unwrap_or_default();
        comment.version_number = version_by_id
            .get(comment.version_id.as_str())
            .and_then(|node| version_number_by_node.get(node))
            .cloned()
            .unwrap_or_default();
    }
    Ok(())
}

pub(super) fn article_ids_of_user(db: &Database, user_id: &str) -> Result<Vec<String>, Error> {
    db.read(|scope| {
        let Some(user) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(Vec::new());
        };
        let mut ids = Vec::new();
        let mut seen = HashSet::new();
        let articles = scope.outgoing(user, EdgeKind::UserAuthorArticle)?;
        for article in articles {
            if has_soft_deleted_flag(scope, article)? {
                continue;
            }
            if let Some(row) = scope.scope_read_node::<IdRow>(article)?
                && seen.insert(row.id.clone())
            {
                ids.push(row.id);
            }
        }

        let comment_edges = scope.outgoing(user, EdgeKind::UserAuthorComment)?;
        for comment in comment_edges {
            if let Some(article_id) = article_id_of_comment(scope, comment)?
                && seen.insert(article_id.clone())
            {
                ids.push(article_id);
            }
        }
        Ok(ids)
    })
}

fn article_id_of_comment(
    scope: &impl GraphRead,
    comment: database::NodeId,
) -> Result<Option<String>, Error> {
    let Some(version_edge) = scope
        .scope_outgoing(comment, EdgeKind::CommentAttachVersion)?
        .first()
        .copied()
    else {
        return Ok(None);
    };
    let Some(article_edge) = scope
        .scope_incoming(version_edge, EdgeKind::ArticleHoldVersion)?
        .first()
        .copied()
    else {
        return Ok(None);
    };
    if has_soft_deleted_flag(scope, article_edge)? {
        return Ok(None);
    }
    Ok(scope
        .scope_read_node::<IdRow>(article_edge)?
        .map(|row| row.id))
}

pub(super) fn all_article_ids(db: &Database) -> Result<Vec<String>, Error> {
    db.read(|scope| {
        let articles = scope.all_nodes(NodeKind::Article)?;
        let mut ids = Vec::with_capacity(articles.len());
        for article in articles {
            if has_soft_deleted_flag(scope, article)? {
                continue;
            }
            if let Some(row) = scope.scope_read_node::<IdRow>(article)? {
                ids.push(row.id);
            }
        }
        Ok(ids)
    })
}
