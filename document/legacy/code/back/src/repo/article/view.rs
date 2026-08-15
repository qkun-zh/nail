
use agdb::{DbError, QueryBuilder};
use serde_json::Value;

use crate::repo::db::{DbHandle, read_rows_sync, resolve_node_ids_sync};
use crate::repo::types::{ArticleRow, AuthorRow, IdRow, TagRow, VersionRow};
use crate::repo::types::{
    EDGE_ARTICLE_TO_TAG, EDGE_USER_TO_ARTICLE, ENTITY_TYPE_ARTICLE, ENTITY_TYPE_VERSION, KEY_TYPE,
};

pub async fn enrich_articles_batch(
    db: &DbHandle,
    article_ids: &[String],
) -> Result<std::collections::HashMap<String, Value>, DbError> {
    if article_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let db = db.read().await;
    use std::collections::HashMap;

    let resolved = resolve_node_ids_sync(&db, ENTITY_TYPE_ARTICLE, article_ids)?;
    let article_nodes: Vec<(String, agdb::DbId)> = article_ids
        .iter()
        .zip(&resolved)
        .filter_map(|(id, node)| node.map(|n| (id.clone(), n)))
        .collect();
    let node_ids: Vec<agdb::DbId> = article_nodes.iter().map(|(_, n)| *n).collect();
    let article_node_set: std::collections::HashSet<agdb::DbId> =
        node_ids.iter().copied().collect();

    let article_rows: Vec<ArticleRow> = read_rows_sync::<ArticleRow>(&db, &node_ids)?;
    let article_row_by_node: HashMap<agdb::DbId, ArticleRow> = article_rows
        .into_iter()
        .filter_map(|r| r.db_id.map(|n| (n, r)))
        .collect();

    let owner_edges = db.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(EDGE_USER_TO_ARTICLE)
            .query(),
    )?;
    let mut owner_of: HashMap<agdb::DbId, agdb::DbId> =
        HashMap::with_capacity(owner_edges.elements.len());
    for edge in &owner_edges.elements {
        if article_node_set.contains(&edge.to) {
            owner_of.insert(edge.to, edge.from);
        }
    }
    let owner_ids: Vec<agdb::DbId> = owner_of.values().copied().collect();
    let author_rows: Vec<AuthorRow> = read_rows_sync::<AuthorRow>(&db, &owner_ids)?;
    let author_by_node: HashMap<agdb::DbId, AuthorRow> = author_rows
        .into_iter()
        .filter_map(|r| r.db_id.map(|n| (n, r)))
        .collect();

    let mut latest_of: HashMap<agdb::DbId, String> = HashMap::new();
    let mut latest_ids: Vec<String> = Vec::new();
    for (_, node) in &article_nodes {
        if let Some(latest) = article_row_by_node
            .get(node)
            .and_then(|r| r.latest_version_id.as_deref())
        {
            latest_of.insert(*node, latest.to_string());
            latest_ids.push(latest.to_string());
        }
    }
    let resolved_versions = resolve_node_ids_sync(&db, ENTITY_TYPE_VERSION, &latest_ids)?;
    let version_node_of_latest: HashMap<String, agdb::DbId> = latest_ids
        .iter()
        .zip(&resolved_versions)
        .filter_map(|(latest, node)| node.map(|n| (latest.clone(), n)))
        .collect();
    let version_ids: Vec<agdb::DbId> = version_node_of_latest.values().copied().collect();
    let version_rows: Vec<VersionRow> = read_rows_sync::<VersionRow>(&db, &version_ids)?;
    let version_by_node: HashMap<agdb::DbId, VersionRow> = version_rows
        .into_iter()
        .filter_map(|r| r.db_id.map(|n| (n, r)))
        .collect();
    let mut version_num_by_latest: HashMap<String, String> = HashMap::new();
    for (latest, vnode) in &version_node_of_latest {
        if let Some(vrow) = version_by_node.get(vnode) {
            version_num_by_latest.insert(latest.clone(), vrow.version_number.clone());
        }
    }

    let tag_edges = db.exec(
        QueryBuilder::search()
            .elements()
            .where_()
            .key(KEY_TYPE)
            .value(EDGE_ARTICLE_TO_TAG)
            .query(),
    )?;
    let tag_target_ids: Vec<agdb::DbId> = tag_edges.elements.iter().map(|e| e.to).collect();
    let tag_rows: Vec<TagRow> = read_rows_sync::<TagRow>(&db, &tag_target_ids)?;
    let tag_id_rows: Vec<IdRow> = read_rows_sync::<IdRow>(&db, &tag_target_ids)?;
    let tag_by_node: HashMap<agdb::DbId, TagRow> = tag_rows
        .into_iter()
        .filter_map(|r| r.db_id.map(|n| (n, r)))
        .collect();
    let id_by_node: HashMap<agdb::DbId, String> = tag_id_rows
        .into_iter()
        .filter_map(|r| r.db_id.map(|n| (n, r.id)))
        .collect();

    let mut out = std::collections::HashMap::with_capacity(article_nodes.len());
    for (article_id, node) in article_nodes {
        let owner = owner_of.get(&node).and_then(|o| author_by_node.get(o));
        let author_name = owner.map(|r| r.name.clone()).unwrap_or_default();
        let author_id = owner.map(|r| r.id.clone()).unwrap_or_default();

        let latest_version_id = latest_of.get(&node).cloned().unwrap_or_default();
        let latest_version = version_num_by_latest
            .get(&latest_version_id)
            .cloned()
            .unwrap_or_default();

        let mut tag_entries: Vec<(u64, Value)> = Vec::new();
        for edge in &tag_edges.elements {
            if edge.from == node
                && let Some(tag) = tag_by_node.get(&edge.to)
                && let Some(id) = id_by_node.get(&edge.to)
            {
                tag_entries.push((
                    edge.to.as_index(),
                    serde_json::json!({ "id": id, "name": tag.tag_name }),
                ));
            }
        }
        tag_entries.sort_by_key(|(index, _)| *index);

        out.insert(
            article_id,
            serde_json::json!({
                "_author": author_name,
                "_author_id": author_id,
                "_latest_version": latest_version,
                "_latest_version_id": latest_version_id,
                "_tags": tag_entries.into_iter().map(|(_, entry)| entry).collect::<Vec<_>>(),
            }),
        );
    }
    Ok(out)
}
