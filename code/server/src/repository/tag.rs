use common::response::NamedRef;
use database::{Database, EdgeKind, Error, NodeKind, Value};

use crate::repository::access::GraphRead;
use crate::repository::schema::{KEY_TAG_NAME, TagRow};

pub fn create_tag_in_scope(
    scope: &mut database::WriteScope<'_, '_>,
    name: &str,
) -> Result<NamedRef, Error> {
    if let Some(existing_id) = find_tag_id_by_name_in_scope(scope, name)? {
        return Ok(NamedRef {
            id: existing_id,
            name: name.to_string(),
        });
    }
    let tag_id = uuid::Uuid::now_v7().to_string();
    scope.insert_node(&TagRow {
        id: tag_id.clone(),
        tag_name: name.to_string(),
    })?;
    Ok(NamedRef {
        id: tag_id,
        name: name.to_string(),
    })
}

fn find_tag_id_by_name_in_scope(
    scope: &impl GraphRead,
    name: &str,
) -> Result<Option<String>, Error> {
    Ok(scope
        .scope_find_by_key(KEY_TAG_NAME, name)?
        .and_then(|id| scope.scope_read_node::<TagRow>(id).transpose())
        .transpose()?
        .map(|row| row.id))
}

pub struct TagViewRow {
    pub id: String,
    pub tag_name: String,
}

pub fn create_tag(db: &Database, name: &str) -> Result<String, Error> {
    db.write(|scope| {
        if scope.find_by_key(KEY_TAG_NAME, name)?.is_some() {
            return Ok(name.to_string());
        }
        let tag_id = uuid::Uuid::now_v7().to_string();
        scope.insert_node(&TagRow {
            id: tag_id.clone(),
            tag_name: name.to_string(),
        })?;
        Ok(tag_id)
    })
}

pub fn read_tag_by_id(db: &Database, tag_id: &str) -> Result<Option<TagViewRow>, Error> {
    db.read(|scope| {
        let Some(db_id) = scope.resolve(NodeKind::Tag, tag_id)? else {
            return Ok(None);
        };
        Ok(scope
            .scope_read_node::<TagRow>(db_id)?
            .map(|row| TagViewRow {
                id: row.id,
                tag_name: row.tag_name,
            }))
    })
}

pub fn read_tag_by_name(db: &Database, name: &str) -> Result<Option<TagViewRow>, Error> {
    db.read(|scope| {
        let Some(id) = scope.find_by_key(KEY_TAG_NAME, name)? else {
            return Ok(None);
        };
        Ok(scope.scope_read_node::<TagRow>(id)?.map(|row| TagViewRow {
            id: row.id,
            tag_name: row.tag_name,
        }))
    })
}

pub fn read_tags(db: &Database) -> Result<Vec<TagViewRow>, Error> {
    db.read(|scope| {
        let nodes = scope.all_nodes(NodeKind::Tag)?;
        let rows = scope.scope_read_nodes::<TagRow>(&nodes)?;
        let mut tags: Vec<TagViewRow> = rows
            .into_iter()
            .map(|row| TagViewRow {
                id: row.id,
                tag_name: row.tag_name,
            })
            .collect();
        tags.sort_by(|left, right| left.tag_name.cmp(&right.tag_name));
        Ok(tags)
    })
}

pub fn update_tag(db: &Database, tag_id: &str, new_name: &str) -> Result<(), Error> {
    db.write(|scope| {
        let Some(db_id) = scope.resolve(NodeKind::Tag, tag_id)? else {
            return Ok(());
        };
        scope.set_key(db_id, KEY_TAG_NAME, Value::Text(new_name.to_string()))?;
        Ok(())
    })
}

pub fn delete_tag(db: &Database, tag_id: &str) -> Result<(), Error> {
    db.write(|scope| {
        let Some(db_id) = scope.resolve(NodeKind::Tag, tag_id)? else {
            return Ok(());
        };
        scope.remove(&[db_id])?;
        Ok(())
    })
}

pub fn count_tag_articles(db: &Database, tag_id: &str) -> Result<u64, Error> {
    db.read(|scope| {
        let Some(db_id) = scope.resolve(NodeKind::Tag, tag_id)? else {
            return Ok(0);
        };
        scope.count_incoming(db_id, EdgeKind::ArticleApplyTag)
    })
}

pub fn apply_tag_to_article(db: &Database, article_id: &str, tag_id: &str) -> Result<(), Error> {
    db.write(|scope| {
        let Some(article_db_id) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(());
        };
        let Some(tag_db_id) = scope.resolve(NodeKind::Tag, tag_id)? else {
            return Ok(());
        };
        scope.insert_edge(
            NodeKind::Article,
            article_db_id,
            EdgeKind::ArticleApplyTag,
            NodeKind::Tag,
            tag_db_id,
        )?;
        Ok(())
    })
}

pub fn unapply_tag_from_article(
    db: &Database,
    article_id: &str,
    tag_id: &str,
) -> Result<(), Error> {
    db.write(|scope| {
        let Some(article_db_id) = scope.resolve(NodeKind::Article, article_id)? else {
            return Ok(());
        };
        let Some(tag_db_id) = scope.resolve(NodeKind::Tag, tag_id)? else {
            return Ok(());
        };
        scope.remove_edge(article_db_id, EdgeKind::ArticleApplyTag, tag_db_id)?;
        Ok(())
    })
}

#[cfg(test)]
pub fn read_tag_articles(db: &Database, tag_id: &str) -> Result<Vec<String>, Error> {
    db.read(|scope| {
        let Some(db_id) = scope.resolve(NodeKind::Tag, tag_id)? else {
            return Ok(Vec::new());
        };
        let articles = scope.incoming(db_id, EdgeKind::ArticleApplyTag)?;
        let rows = scope.scope_read_nodes::<crate::repository::schema::IdRow>(&articles)?;
        Ok(rows.into_iter().map(|row| row.id).collect())
    })
}
