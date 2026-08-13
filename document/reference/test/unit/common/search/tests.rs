
use super::*;

fn full_params() -> ArticleSearchParams {
    ArticleSearchParams {
        q: Some("memory rust".to_string()),
        ranges: Some("title,comment,note".to_string()),
        sort: Some("time:desc,title:asc".to_string()),
        from: Some(1_700_000_000),
        to: Some(1_800_000_000),
        limit: Some(20),
        page: Some(3),
    }
}

#[test]
fn default_params_are_all_none() {
    let json = serde_json::to_string(&ArticleSearchParams::default()).unwrap();
    let obj: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(obj.len(), 7, "all 7 params serialize even when None");
    assert!(
        obj.values().all(|v| v.is_null()),
        "every field must be null: {json}"
    );
}

#[test]
fn full_params_wire_shape() {
    let json = serde_json::to_string(&full_params()).unwrap();
    let obj: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&json).unwrap();
    for key in ["q", "ranges", "sort", "from", "to", "limit", "page"] {
        assert!(obj.contains_key(key), "missing wire key {key:?} in {json}");
    }
    assert_eq!(obj["ranges"], "title,comment,note");
    assert_eq!(obj["sort"], "time:desc,title:asc");
    assert_eq!(obj["q"], "memory rust");
}

#[test]
fn full_params_roundtrip() {
    let json = serde_json::to_string(&full_params()).unwrap();
    let back: ArticleSearchParams = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn partial_params_roundtrip_preserves_only_present_keys() {
    let partial = ArticleSearchParams {
        ranges: Some("#a#b".to_string()),
        page: Some(7),
        ..Default::default()
    };
    let json = serde_json::to_string(&partial).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["ranges"], "#a#b");
    assert_eq!(v["page"], 7);
    let back: ArticleSearchParams = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}