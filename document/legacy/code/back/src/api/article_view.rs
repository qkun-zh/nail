
use crate::api::strip_table_prefix;
use crate::other::AppState;
use crate::repo::article::enrich_articles_batch;
use std::collections::HashMap;

pub(crate) fn extract_record_id(article: &serde_json::Value) -> String {
    article
        .get("id")
        .and_then(|v| v.as_str())
        .map(strip_table_prefix)
        .unwrap_or_default()
}

pub(crate) fn normalize_tag_rows(tags: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    tags.into_iter()
        .filter_map(|tag| {
            let name = tag.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let id = tag
                .get("id")
                .and_then(|v| v.as_str())
                .map(strip_table_prefix)
                .unwrap_or_default();
            if name.is_empty() || id.is_empty() {
                return None;
            }
            Some(serde_json::json!({ "id": id, "name": name }))
        })
        .collect()
}

pub(crate) async fn build_article_view(
    state: &AppState,
    article: serde_json::Value,
) -> serde_json::Value {
    let clean_id = extract_record_id(&article);

    let enriched = match enrich_articles_batch(&state.db, std::slice::from_ref(&clean_id)).await {
        Ok(mut map) => map.remove(&clean_id).unwrap_or_default(),
        Err(e) => {
            tracing::warn!(target: "article", error = %e, article_id = %clean_id,
                "best-effort enrichment failed; degrading to no author/tags");
            serde_json::Value::Object(serde_json::Map::new())
        }
    };

    let author_id = enriched
        .get("_author_id")
        .and_then(|v| v.as_str())
        .map(strip_table_prefix)
        .unwrap_or_default();
    let author_name = enriched
        .get("_author")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let title = article
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let summary = article
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let created_at = common::time::uuidv7_timestamp_secs(&clean_id).unwrap_or(0);
    let tags = normalize_tag_rows(
        enriched
            .get("_tags")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
    );

    let view = serde_json::json!({
        "id": clean_id,
        "author_id": author_id,
        "author_name": author_name,
        "title": title,
        "summary": summary,
        "created_at": created_at,
        "tags": tags
    });

    view
}

pub(crate) async fn build_article_views(
    state: &AppState,
    article_list: Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    if article_list.is_empty() {
        return Vec::new();
    }

    let ids: Vec<String> = article_list.iter().map(extract_record_id).collect();
    let enrich_map = match enrich_articles_batch(&state.db, &ids).await {
        Ok(map) => map,
        Err(e) => {
            tracing::warn!(target: "article", error = %e,
                "best-effort enrichment failed; degrading to no author/tags");
            HashMap::new()
        }
    };

    article_list
        .into_iter()
        .filter_map(|article| {
            let id = extract_record_id(&article);
            let enriched = enrich_map.get(&id).cloned().unwrap_or_default();
            let serde_json::Value::Object(mut map) = article else {
                return None;
            };
            map.remove("_author");
            map.remove("_latest_version_id");

            let author_id = enriched
                .get("_author_id")
                .and_then(|v| v.as_str())
                .map(strip_table_prefix)
                .unwrap_or_default();
            let author_name = enriched
                .get("_author")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if let Some(version) = enriched.get("_latest_version").and_then(|v| v.as_str()) {
                map.insert(
                    "latest_version".to_string(),
                    serde_json::Value::String(version.to_string()),
                );
            }
            if let Some(version_id) = enriched.get("_latest_version_id").and_then(|v| v.as_str()) {
                map.insert(
                    "latest_version_id".to_string(),
                    serde_json::Value::String(strip_table_prefix(version_id)),
                );
            }
            map.insert(
                "author_id".to_string(),
                serde_json::Value::String(author_id),
            );
            map.insert(
                "author_name".to_string(),
                serde_json::Value::String(author_name),
            );
            map.insert(
                "tags".to_string(),
                serde_json::Value::Array(normalize_tag_rows(
                    enriched
                        .get("_tags")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default(),
                )),
            );
            map.insert("id".to_string(), serde_json::Value::String(id));
            Some(serde_json::Value::Object(map))
        })
        .collect()
}
