use crate::response::ResponseEnvelope;

#[test]
fn ok_constructor_carries_code_data_and_message() {
    let envelope: ResponseEnvelope<u64> = ResponseEnvelope::ok(200, 42, "ok");
    assert_eq!(envelope.code, 200);
    assert_eq!(envelope.data, Some(42));
    assert_eq!(envelope.message, "ok");
}

#[test]
fn err_constructor_carries_code_and_message_with_null_data() {
    let envelope: ResponseEnvelope<u64> = ResponseEnvelope::err(404, "article not found");
    assert_eq!(envelope.code, 404);
    assert_eq!(envelope.data, None);
    assert_eq!(envelope.message, "article not found");
}

#[test]
fn ok_envelope_serializes_with_data_present() {
    let envelope = ResponseEnvelope::ok(200, 42u64, "ok");
    let json = serde_json::to_string(&envelope).expect("serialize ok envelope");
    assert_eq!(json, r##"{"code":200,"data":42,"message":"ok"}"##);
}

#[test]
fn err_envelope_serializes_with_null_data() {
    let envelope: ResponseEnvelope<u64> = ResponseEnvelope::err(404, "article not found");
    let json = serde_json::to_string(&envelope).expect("serialize err envelope");
    assert_eq!(
        json,
        r##"{"code":404,"data":null,"message":"article not found"}"##
    );
}

#[test]
fn envelope_round_trips_through_json() {
    let original = ResponseEnvelope::ok(201, vec!["a".to_string(), "b".to_string()], "created");
    let json = serde_json::to_string(&original).expect("serialize envelope");
    let decoded: ResponseEnvelope<Vec<String>> =
        serde_json::from_str(&json).expect("deserialize envelope");
    assert_eq!(decoded.code, original.code);
    assert_eq!(decoded.data, original.data);
    assert_eq!(decoded.message, original.message);
}

#[test]
fn search_hit_round_trips_with_lowercase_field() -> anyhow::Result<()> {
    let hit = crate::response::search::SearchHit {
        field: crate::search::SearchRange::Title,
        label: "Title".to_string(),
        snippet: "Rust <mark>borrow</mark> checker".to_string(),
    };
    let json = serde_json::to_string(&hit)?;
    assert_eq!(
        json,
        r##"{"field":"title","label":"Title","snippet":"Rust <mark>borrow</mark> checker"}"##
    );
    let decoded: crate::response::search::SearchHit = serde_json::from_str(&json)?;
    assert_eq!(decoded, hit);
    Ok(())
}

#[test]
fn search_article_item_round_trips_with_hits() -> anyhow::Result<()> {
    let item = crate::response::search::SearchArticleItem {
        id: "0197c0b0-1234-7000-8000-000000000001".to_string(),
        title: "Rust borrow checker".to_string(),
        author: "alice".to_string(),
        time: "2023-11-15T06:13:20+08:00".to_string(),
        hits: vec![crate::response::search::SearchHit {
            field: crate::search::SearchRange::Summary,
            label: "Summary".to_string(),
            snippet: "A <mark>summary</mark>".to_string(),
        }],
    };
    let json = serde_json::to_string(&item)?;
    let decoded: crate::response::search::SearchArticleItem = serde_json::from_str(&json)?;
    assert_eq!(decoded, item);
    Ok(())
}

#[test]
fn search_page_round_trips_with_paging_fields() -> anyhow::Result<()> {
    let page = crate::response::search::SearchPage {
        article_list: Vec::new(),
        total: 0,
        page: 1,
        total_pages: 1,
        has_next: false,
        has_prev: false,
        truncated: false,
    };
    let json = serde_json::to_string(&page)?;
    let decoded: crate::response::search::SearchPage = serde_json::from_str(&json)?;
    assert_eq!(decoded, page);
    Ok(())
}
