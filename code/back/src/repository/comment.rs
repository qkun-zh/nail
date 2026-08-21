use database::{Database, EdgeKind, Error, NodeId, NodeKind};

use crate::repository::access::GraphRead;
use crate::repository::delete::has_soft_deleted_flag;
use crate::repository::schema::{CommentRow, IdRow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentTreeItem {
    pub id: String,
    pub content: String,
    pub author_id: String,
    pub parent_id: Option<String>,
    pub child_count: u64,
}

#[derive(Debug)]
pub enum CreateCommentError {
    TargetNotFound,
    CommentIdExists,
    CommentTreeTooDeep,
    Db(Error),
}

impl From<Error> for CreateCommentError {
    fn from(error: Error) -> Self {
        Self::Db(error)
    }
}

impl std::fmt::Display for CreateCommentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetNotFound => formatter.write_str("comment target not found"),
            Self::CommentIdExists => formatter.write_str("comment id already exists"),
            Self::CommentTreeTooDeep => formatter.write_str("comment thread too deep"),
            Self::Db(error) => write!(formatter, "database query failed: {error}"),
        }
    }
}

impl std::error::Error for CreateCommentError {}

pub fn create_top_level_comment(
    db: &Database,
    comment_id: &str,
    user_id: &str,
    version_id: &str,
    content: &str,
) -> Result<(), CreateCommentError> {
    db.write(|scope| {
        let Some(user) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(Err(CreateCommentError::TargetNotFound));
        };
        let Some(version) = scope.resolve(NodeKind::Version, version_id)? else {
            return Ok(Err(CreateCommentError::TargetNotFound));
        };
        if has_soft_deleted_flag(scope, version)? {
            return Ok(Err(CreateCommentError::TargetNotFound));
        }
        if scope.resolve(NodeKind::Comment, comment_id)?.is_some() {
            return Ok(Err(CreateCommentError::CommentIdExists));
        }
        scope.insert_node(&CommentRow {
            id: comment_id.to_string(),
            content: content.to_string(),
        })?;
        let comment = scope
            .resolve(NodeKind::Comment, comment_id)?
            .ok_or_else(|| Error::Invalid("inserted comment missing".to_string()))?;
        scope.insert_edge(
            NodeKind::User,
            user,
            EdgeKind::UserAuthorComment,
            NodeKind::Comment,
            comment,
        )?;
        scope.insert_edge(
            NodeKind::Comment,
            comment,
            EdgeKind::CommentAttachVersion,
            NodeKind::Version,
            version,
        )?;
        Ok(Ok(()))
    })
    .map_err(CreateCommentError::from)
    .and_then(std::convert::identity)
}

pub fn create_reply_comment(
    db: &Database,
    comment_id: &str,
    user_id: &str,
    parent_comment_id: &str,
    content: &str,
    max_tree_depth: usize,
) -> Result<(), CreateCommentError> {
    db.write(|scope| {
        let Some(user) = scope.resolve(NodeKind::User, user_id)? else {
            return Ok(Err(CreateCommentError::TargetNotFound));
        };
        let Some(parent) = scope.resolve(NodeKind::Comment, parent_comment_id)? else {
            return Ok(Err(CreateCommentError::TargetNotFound));
        };
        if has_soft_deleted_flag(scope, parent)? {
            return Ok(Err(CreateCommentError::TargetNotFound));
        }
        if parent_chain_depth_in_scope(scope, parent_comment_id, max_tree_depth)? >= max_tree_depth
        {
            return Ok(Err(CreateCommentError::CommentTreeTooDeep));
        }
        if scope.resolve(NodeKind::Comment, comment_id)?.is_some() {
            return Ok(Err(CreateCommentError::CommentIdExists));
        }
        scope.insert_node(&CommentRow {
            id: comment_id.to_string(),
            content: content.to_string(),
        })?;
        let comment = scope
            .resolve(NodeKind::Comment, comment_id)?
            .ok_or_else(|| Error::Invalid("inserted comment missing".to_string()))?;
        scope.insert_edge(
            NodeKind::User,
            user,
            EdgeKind::UserAuthorComment,
            NodeKind::Comment,
            comment,
        )?;
        scope.insert_edge(
            NodeKind::Comment,
            comment,
            EdgeKind::CommentReplyComment,
            NodeKind::Comment,
            parent,
        )?;
        Ok(Ok(()))
    })
    .map_err(CreateCommentError::from)
    .and_then(std::convert::identity)
}

fn parent_chain_depth_in_scope(
    scope: &impl GraphRead,
    comment_id: &str,
    max_tree_depth: usize,
) -> Result<usize, Error> {
    let mut depth = 0usize;
    let mut current = comment_id.to_string();
    loop {
        let Some(current_node) = scope.scope_resolve(NodeKind::Comment, &current)? else {
            return Ok(depth);
        };
        let Some(parent_node) = scope
            .scope_outgoing(current_node, EdgeKind::CommentReplyComment)?
            .first()
            .copied()
        else {
            return Ok(depth);
        };
        let Some(parent_id) = scope
            .scope_read_node::<IdRow>(parent_node)?
            .map(|row| row.id)
        else {
            return Ok(depth);
        };
        current = parent_id;
        depth += 1;
        if depth > max_tree_depth {
            return Ok(depth);
        }
    }
}

pub fn owner_of_comment(db: &Database, comment_id: &str) -> Result<Option<String>, Error> {
    db.read(|scope| {
        let Some(comment) = scope.resolve(NodeKind::Comment, comment_id)? else {
            return Ok(None);
        };
        Ok(scope
            .incoming(comment, EdgeKind::UserAuthorComment)?
            .first()
            .and_then(|user| scope.scope_read_node::<IdRow>(*user).transpose())
            .transpose()?
            .map(|row| row.id))
    })
}

pub fn count_comments_by_version(db: &Database, version_id: &str) -> Result<u64, Error> {
    db.read(|scope| {
        let Some(version) = scope.resolve(NodeKind::Version, version_id)? else {
            return Ok(0);
        };
        if has_soft_deleted_flag(scope, version)? {
            return Ok(0);
        }
        count_incoming_comments(scope, version)
    })
}

pub fn count_comment_children(db: &Database, parent_comment_id: &str) -> Result<u64, Error> {
    db.read(|scope| {
        let Some(parent) = scope.resolve(NodeKind::Comment, parent_comment_id)? else {
            return Ok(0);
        };
        count_incoming_comments(scope, parent)
    })
}

fn count_incoming_comments(scope: &impl GraphRead, node: NodeId) -> Result<u64, Error> {
    let comments = scope.scope_incoming(node, EdgeKind::CommentAttachVersion)?;
    let mut count = 0u64;
    for comment in comments {
        if !has_soft_deleted_flag(scope, comment)? {
            count += 1;
        }
    }
    Ok(count)
}

pub fn read_comments_page_by_version(
    db: &Database,
    version_id: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<CommentTreeItem>, bool), Error> {
    db.read(|scope| {
        let Some(version) = scope.resolve(NodeKind::Version, version_id)? else {
            return Ok((Vec::new(), false));
        };
        if has_soft_deleted_flag(scope, version)? {
            return Ok((Vec::new(), false));
        }
        read_comments_page(scope, version, limit, offset)
    })
}

fn read_comments_page(
    scope: &impl GraphRead,
    version: NodeId,
    limit: u64,
    offset: u64,
) -> Result<(Vec<CommentTreeItem>, bool), Error> {
    let (page_ids, has_next) = incoming_comment_ids_page(
        scope,
        version,
        EdgeKind::CommentAttachVersion,
        limit,
        offset,
    )?;
    let items = read_comment_items(scope, &page_ids)?;
    Ok((items, has_next))
}

pub fn read_comment_children_page(
    db: &Database,
    parent_comment_id: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<CommentTreeItem>, bool), Error> {
    db.read(|scope| {
        let Some(parent) = scope.resolve(NodeKind::Comment, parent_comment_id)? else {
            return Err(Error::NotFound {
                kind: NodeKind::Comment,
                id: parent_comment_id.to_string(),
            });
        };
        if has_soft_deleted_flag(scope, parent)? {
            return Err(Error::NotFound {
                kind: NodeKind::Comment,
                id: parent_comment_id.to_string(),
            });
        }
        let (page_ids, has_next) =
            incoming_comment_ids_page(scope, parent, EdgeKind::CommentReplyComment, limit, offset)?;
        let items = read_comment_items(scope, &page_ids)?;
        Ok((items, has_next))
    })
}

pub fn read_comment_item(
    db: &Database,
    comment_id: &str,
) -> Result<Option<CommentTreeItem>, Error> {
    db.read(|scope| read_comment_item_any(scope, comment_id))
}

fn incoming_comment_ids_page(
    scope: &impl GraphRead,
    node: NodeId,
    edge_kind: EdgeKind,
    limit: u64,
    offset: u64,
) -> Result<(Vec<String>, bool), Error> {
    let comments = scope.scope_incoming(node, edge_kind)?;
    let mut live = Vec::new();
    for comment in comments {
        if !has_soft_deleted_flag(scope, comment)? {
            live.push(comment);
        }
    }
    let has_next = (live.len() as u64) > offset + limit;
    let skip = usize::try_from(offset).unwrap_or(usize::MAX);
    let take = usize::try_from(limit).unwrap_or(usize::MAX);
    let page: Vec<String> = live
        .drain(..)
        .skip(skip)
        .take(take)
        .filter_map(|comment| scope.scope_read_node::<IdRow>(comment).ok().flatten())
        .map(|row| row.id)
        .collect();
    Ok((page, has_next))
}

fn child_count(scope: &impl GraphRead, comment: NodeId) -> Result<u64, Error> {
    scope.scope_count_incoming(comment, EdgeKind::CommentReplyComment)
}

fn read_comment_item_any(
    scope: &impl GraphRead,
    comment_id: &str,
) -> Result<Option<CommentTreeItem>, Error> {
    let Some(comment) = scope.scope_resolve(NodeKind::Comment, comment_id)? else {
        return Ok(None);
    };
    let content = scope
        .scope_read_node::<CommentRow>(comment)?
        .map(|row| row.content)
        .unwrap_or_default();
    let author_id = scope
        .scope_incoming(comment, EdgeKind::UserAuthorComment)?
        .first()
        .and_then(|user| scope.scope_read_node::<IdRow>(*user).ok().flatten())
        .map(|row| row.id)
        .unwrap_or_default();
    let parent_id = scope
        .scope_outgoing(comment, EdgeKind::CommentReplyComment)?
        .first()
        .and_then(|parent| scope.scope_read_node::<IdRow>(*parent).ok().flatten())
        .map(|row| row.id);
    let child_count = child_count(scope, comment)?;
    Ok(Some(CommentTreeItem {
        id: comment_id.to_string(),
        content,
        author_id,
        parent_id,
        child_count,
    }))
}

fn read_comment_item_filtered(
    scope: &impl GraphRead,
    comment_id: &str,
) -> Result<Option<CommentTreeItem>, Error> {
    let Some(comment) = scope.scope_resolve(NodeKind::Comment, comment_id)? else {
        return Ok(None);
    };
    if has_soft_deleted_flag(scope, comment)? {
        return Ok(None);
    }
    read_comment_item_any(scope, comment_id)
}

fn read_comment_items(
    scope: &impl GraphRead,
    comment_ids: &[String],
) -> Result<Vec<CommentTreeItem>, Error> {
    let mut items = Vec::with_capacity(comment_ids.len());
    for comment_id in comment_ids {
        if let Some(item) = read_comment_item_filtered(scope, comment_id)? {
            items.push(item);
        }
    }
    Ok(items)
}

pub fn update_comment_content(
    db: &Database,
    comment_id: &str,
    content: &str,
) -> Result<bool, Error> {
    db.write(|scope| {
        let Some(node) = scope.resolve(NodeKind::Comment, comment_id)? else {
            return Ok(false);
        };
        if has_soft_deleted_flag(scope, node)? {
            return Ok(false);
        }
        scope.set_key(
            node,
            crate::repository::schema::KEY_COMMENT_CONTENT,
            database::Value::Text(content.to_string()),
        )?;
        Ok(true)
    })
}

pub fn version_of_comment(db: &Database, comment_id: &str) -> Result<Option<String>, Error> {
    db.read(|scope| {
        let mut current = comment_id.to_string();
        loop {
            let Some(comment) = scope.resolve(NodeKind::Comment, &current)? else {
                return Ok(None);
            };
            if let Some(parent_node) = scope
                .outgoing(comment, EdgeKind::CommentReplyComment)?
                .first()
                .copied()
                && let Some(parent_id) = scope
                    .scope_read_node::<IdRow>(parent_node)?
                    .map(|row| row.id)
            {
                current = parent_id;
                continue;
            }
            return Ok(scope
                .outgoing(comment, EdgeKind::CommentAttachVersion)?
                .first()
                .and_then(|version| scope.scope_read_node::<IdRow>(*version).transpose())
                .transpose()?
                .map(|row| row.id));
        }
    })
}
