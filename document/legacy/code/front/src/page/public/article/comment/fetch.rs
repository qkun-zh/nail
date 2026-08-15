
use anyhow::Context;
use std::collections::{HashMap, HashSet};

pub fn build_comment_tree(
    comments: &[serde_json::Value],
) -> (
    Vec<&serde_json::Value>,
    HashMap<&str, Vec<&serde_json::Value>>,
) {
    let ids: HashSet<&str> = comments
        .iter()
        .filter_map(|c| c.get("id").and_then(|v| v.as_str()))
        .collect();
    let mut children: HashMap<&str, Vec<&serde_json::Value>> = HashMap::new();
    let mut roots: Vec<&serde_json::Value> = Vec::new();
    for comment in comments {
        match comment.get("parent_id") {
            Some(parent) if parent.is_string() => {
                let parent = parent.as_str().unwrap_or_default();
                if ids.contains(parent) {
                    children.entry(parent).or_default().push(comment);
                }
            }
            None | Some(serde_json::Value::Null) => roots.push(comment),
            Some(_) => {}
        }
    }
    (roots, children)
}

pub async fn fetch_version_comments(
    version_id: &str,
    page: u64,
    limit: u64,
) -> anyhow::Result<serde_json::Value> {
    crate::req::read_version_comments(version_id, page, limit)
        .await
        .with_context(|| format!("fetch comments for version {version_id}"))
}
