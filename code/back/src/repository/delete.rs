use database::{Database, EdgeKind, Error, NodeId, NodeKind, Value, WriteScope};

use crate::repository::access::GraphRead;
use crate::repository::schema::{KEY_LATEST_VERSION_ID, KEY_SOFT_DELETED, VersionRow};

#[derive(Debug, Default)]
pub struct DeleteOutcome {
    pub removed_pdf_hashes: Vec<String>,
}

pub fn delete_user(db: &Database, user_id: &str) -> Result<DeleteOutcome, Error> {
    db.write(|scope| {
        let mut outcome = DeleteOutcome::default();
        let Some(user) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(outcome);
        };
        for article in scope.outgoing(user, EdgeKind::UserAuthorArticle)? {
            delete_article_in_scope(scope, article, &mut outcome)?;
        }
        for comment in scope.outgoing(user, EdgeKind::UserAuthorComment)? {
            delete_comment_tree_in_scope(scope, comment)?;
        }
        scope.remove(&[user])?;
        Ok(outcome)
    })
}

pub fn delete_article(db: &Database, article_id: &str) -> Result<DeleteOutcome, Error> {
    db.write(|scope| {
        let mut outcome = DeleteOutcome::default();
        let Some(article) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(outcome);
        };
        delete_article_in_scope(scope, article, &mut outcome)?;
        Ok(outcome)
    })
}

pub fn delete_comment(db: &Database, comment_id: &str) -> Result<DeleteOutcome, Error> {
    db.write(|scope| {
        let outcome = DeleteOutcome::default();
        let Some(comment) = scope.resolve(NodeKind::Comment, comment_id)? else {
            return Ok(outcome);
        };
        delete_comment_tree_in_scope(scope, comment)?;
        Ok(outcome)
    })
}

pub fn delete_version(db: &Database, version_id: &str) -> Result<DeleteOutcome, Error> {
    db.write(|scope| {
        let mut outcome = DeleteOutcome::default();
        let Some(version) = scope.resolve(NodeKind::Version, version_id)? else {
            return Ok(outcome);
        };
        let article = scope
            .incoming(version, EdgeKind::ArticleHoldVersion)?
            .first()
            .copied();
        delete_version_in_scope(scope, version, &mut outcome)?;
        if let Some(article) = article {
            refresh_latest_version_in_scope(scope, article)?;
        }
        Ok(outcome)
    })
}

pub(crate) fn has_soft_deleted_flag(scope: &impl GraphRead, id: NodeId) -> Result<bool, Error> {
    Ok(soft_delete_count(scope, id)? > 0)
}

fn soft_delete_count(scope: &impl GraphRead, id: NodeId) -> Result<i64, Error> {
    Ok(scope
        .scope_read_node::<CounterRow>(id)?
        .and_then(|row| row.soft_deleted)
        .unwrap_or(0))
}

pub fn soft_delete_article(db: &Database, article_id: &str) -> Result<(), Error> {
    adjust_soft_delete_count(db, NodeKind::Article, article_id, 1)
}

pub fn soft_delete_version(db: &Database, version_id: &str) -> Result<(), Error> {
    adjust_soft_delete_count(db, NodeKind::Version, version_id, 1)
}

pub fn soft_delete_comment(db: &Database, comment_id: &str) -> Result<(), Error> {
    adjust_soft_delete_count(db, NodeKind::Comment, comment_id, 1)
}

pub fn soft_delete_user(db: &Database, user_id: &str) -> Result<(), Error> {
    adjust_soft_delete_count(db, NodeKind::User, user_id, 1)
}

pub fn undelete_soft_user(db: &Database, user_id: &str) -> Result<(), Error> {
    adjust_soft_delete_count(db, NodeKind::User, user_id, -1)
}

pub fn clear_soft_deleted_flag(db: &Database, business_id: &str) -> Result<(), Error> {
    db.write(|scope| {
        for kind in [NodeKind::Article, NodeKind::Version, NodeKind::Comment] {
            if let Some(id) = scope.resolve(kind, business_id)? {
                return adjust_subtree_in_scope(scope, kind, id, -1);
            }
        }
        Ok(())
    })
}

pub fn is_soft_deleted(db: &Database, kind: NodeKind, business_id: &str) -> Result<bool, Error> {
    db.read(|scope| {
        let Some(id) = scope.resolve(kind, business_id)? else {
            return Ok(false);
        };
        has_soft_deleted_flag(scope, id)
    })
}

fn adjust_soft_delete_count(
    db: &Database,
    kind: NodeKind,
    business_id: &str,
    delta: i64,
) -> Result<(), Error> {
    db.write(|scope| {
        let Some(id) = scope.resolve(kind, business_id)? else {
            return Ok(());
        };
        adjust_subtree_in_scope(scope, kind, id, delta)
    })
}

fn adjust_subtree_in_scope(
    scope: &mut WriteScope<'_, '_>,
    kind: NodeKind,
    id: NodeId,
    delta: i64,
) -> Result<(), Error> {
    match kind {
        NodeKind::Article => adjust_article_subtree(scope, id, delta),
        NodeKind::Version => adjust_version_subtree(scope, id, delta),
        NodeKind::Comment => adjust_comment_tree(scope, id, delta),
        NodeKind::User => adjust_user_subtree(scope, id, delta),
        NodeKind::Tag | NodeKind::Role | NodeKind::Permission => Ok(()),
    }
}

fn adjust_user_subtree(
    scope: &mut WriteScope<'_, '_>,
    user: NodeId,
    delta: i64,
) -> Result<(), Error> {
    for article in scope.outgoing(user, EdgeKind::UserAuthorArticle)? {
        adjust_article_subtree(scope, article, delta)?;
    }
    for comment in scope.outgoing(user, EdgeKind::UserAuthorComment)? {
        adjust_comment_tree(scope, comment, delta)?;
    }
    adjust_node_counter(scope, user, delta)
}

fn adjust_article_subtree(
    scope: &mut WriteScope<'_, '_>,
    article: NodeId,
    delta: i64,
) -> Result<(), Error> {
    for version in scope.outgoing(article, EdgeKind::ArticleHoldVersion)? {
        adjust_version_subtree(scope, version, delta)?;
    }
    adjust_node_counter(scope, article, delta)
}

fn adjust_version_subtree(
    scope: &mut WriteScope<'_, '_>,
    version: NodeId,
    delta: i64,
) -> Result<(), Error> {
    for comment in scope.incoming(version, EdgeKind::CommentAttachVersion)? {
        adjust_comment_tree(scope, comment, delta)?;
    }
    adjust_node_counter(scope, version, delta)
}

fn adjust_comment_tree(
    scope: &mut WriteScope<'_, '_>,
    comment: NodeId,
    delta: i64,
) -> Result<(), Error> {
    for reply in scope.incoming(comment, EdgeKind::CommentReplyComment)? {
        adjust_comment_tree(scope, reply, delta)?;
    }
    adjust_node_counter(scope, comment, delta)
}

fn adjust_node_counter(
    scope: &mut WriteScope<'_, '_>,
    id: NodeId,
    delta: i64,
) -> Result<(), Error> {
    let next = soft_delete_count(scope, id)?.saturating_add(delta);
    if next <= 0 {
        scope.clear_key(id, KEY_SOFT_DELETED)?;
        return Ok(());
    }
    scope.set_key(id, KEY_SOFT_DELETED, Value::Int(next))?;
    Ok(())
}

/// Counter-only projection; every kind carries at most this one metadata key.
struct CounterRow {
    soft_deleted: Option<i64>,
}

impl database::Row for CounterRow {
    const KIND: NodeKind = NodeKind::User;

    fn business_id(&self) -> &'static str {
        ""
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        Vec::new()
    }

    fn from_lookup(lookup: &dyn database::ValueLookup) -> Result<Self, Error> {
        Ok(Self {
            soft_deleted: lookup.get(KEY_SOFT_DELETED).map(|value| match value {
                Value::Int(int) => int,
                Value::Text(_) => 0,
            }),
        })
    }
}

fn delete_article_in_scope(
    scope: &mut WriteScope<'_, '_>,
    article: NodeId,
    outcome: &mut DeleteOutcome,
) -> Result<(), Error> {
    for version in scope.outgoing(article, EdgeKind::ArticleHoldVersion)? {
        delete_version_in_scope(scope, version, outcome)?;
    }
    scope.remove(&[article])?;
    Ok(())
}

fn delete_version_in_scope(
    scope: &mut WriteScope<'_, '_>,
    version: NodeId,
    outcome: &mut DeleteOutcome,
) -> Result<(), Error> {
    for comment in scope.incoming(version, EdgeKind::CommentAttachVersion)? {
        delete_comment_tree_in_scope(scope, comment)?;
    }
    if let Some(row) = scope.scope_read_node::<VersionRow>(version)? {
        outcome.removed_pdf_hashes.push(row.content_hash);
    }
    scope.remove(&[version])?;
    Ok(())
}

fn delete_comment_tree_in_scope(
    scope: &mut WriteScope<'_, '_>,
    comment: NodeId,
) -> Result<(), Error> {
    for reply in scope.incoming(comment, EdgeKind::CommentReplyComment)? {
        delete_comment_tree_in_scope(scope, reply)?;
    }
    scope.remove(&[comment])?;
    Ok(())
}

fn refresh_latest_version_in_scope(
    scope: &mut WriteScope<'_, '_>,
    article: NodeId,
) -> Result<(), Error> {
    let versions = scope.outgoing(article, EdgeKind::ArticleHoldVersion)?;
    let rows = scope.scope_read_nodes::<VersionRow>(&versions)?;
    let latest = highest_version_number(rows).map(|row| row.id);
    match latest {
        Some(id) => scope.set_key(article, KEY_LATEST_VERSION_ID, Value::Text(id)),
        None => scope.clear_key(article, KEY_LATEST_VERSION_ID),
    }
}

pub(crate) fn highest_version_number(rows: Vec<VersionRow>) -> Option<VersionRow> {
    rows.into_iter().max_by(|left, right| {
        let left_version = semver::Version::parse(&left.version_number);
        let right_version = semver::Version::parse(&right.version_number);
        match (left_version, right_version) {
            (Ok(left), Ok(right)) => left.cmp(&right),
            _ => left.version_number.cmp(&right.version_number),
        }
    })
}
