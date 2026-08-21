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
    assert_eq!(json, r#"{"code":200,"data":42,"message":"ok"}"#);
}

#[test]
fn err_envelope_serializes_with_null_data() {
    let envelope: ResponseEnvelope<u64> = ResponseEnvelope::err(404, "article not found");
    let json = serde_json::to_string(&envelope).expect("serialize err envelope");
    assert_eq!(
        json,
        r#"{"code":404,"data":null,"message":"article not found"}"#
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
        r#"{"field":"title","label":"Title","snippet":"Rust <mark>borrow</mark> checker"}"#
    );
    let decoded: crate::response::search::SearchHit = serde_json::from_str(&json)?;
    assert_eq!(decoded, hit);
    Ok(())
}

#[test]
fn search_article_item_round_trips_with_hits() -> anyhow::Result<()> {
    let item = crate::response::search::SearchArticleItem {
        article_id: "0197c0b0-1234-7000-8000-000000000001".to_string(),
        title: "Rust borrow checker".to_string(),
        author_name: "alice".to_string(),
        author_id: "0197c0b0-aaaa-7000-8000-00000000000a".to_string(),
        time: "2023-11-15T06:13:20Z".to_string(),
        article_hits: vec![crate::response::search::SearchHit {
            field: crate::search::SearchRange::Summary,
            label: "summary".to_string(),
            snippet: "A <mark>summary</mark>".to_string(),
        }],
        versions: vec![crate::response::search::SearchVersionItem {
            version_id: "0197c0b0-5678-7000-8000-000000000002".to_string(),
            version_number: "2.1.0".to_string(),
            time: "2023-11-15T06:13:20Z".to_string(),
            version_hits: vec![crate::response::search::SearchHit {
                field: crate::search::SearchRange::Note,
                label: "note".to_string(),
                snippet: "fixes <mark>leak</mark>".to_string(),
            }],
            comments: vec![crate::response::search::SearchCommentItem {
                comment_id: "0197c0b0-9abc-7000-8000-000000000003".to_string(),
                author_name: "bob".to_string(),
                author_id: "0197c0b0-bbbb-7000-8000-00000000000b".to_string(),
                time: "2023-11-15T06:13:20Z".to_string(),
                content: "great <mark>fix</mark>".to_string(),
            }],
        }],
    };
    let json = serde_json::to_string(&item)?;
    let decoded: crate::response::search::SearchArticleItem = serde_json::from_str(&json)?;
    assert_eq!(decoded, item);
    Ok(())
}

#[test]
fn list_page_round_trips_with_items_has_next_and_total() -> anyhow::Result<()> {
    let page = crate::response::ListPage {
        items: Vec::<crate::response::tag::TagListItem>::new(),
        has_next: false,
        total: 3,
    };
    let json = serde_json::to_string(&page)?;
    assert_eq!(json, r#"{"items":[],"has_next":false,"total":3}"#);
    let decoded: crate::response::ListPage<crate::response::tag::TagListItem> =
        serde_json::from_str(&json)?;
    assert_eq!(decoded, page);
    Ok(())
}

#[test]
fn list_page_serializes_items_field_for_each_item_type() -> anyhow::Result<()> {
    for page in [
        serde_json::json!({"items": [], "has_next": false, "total": 0}),
        serde_json::json!({"items": [], "has_next": true, "total": 5}),
    ] {
        let decoded: crate::response::ListPage<crate::response::user::UserListItem> =
            serde_json::from_value(page)?;
        assert!(decoded.items.is_empty());
    }
    Ok(())
}

#[test]
fn session_view_omits_absent_fields() -> anyhow::Result<()> {
    let view = crate::response::session::SessionView {
        id: Some("0197c0b0-1234-7000-8000-000000000001".to_string()),
        name: None,
    };
    let json = serde_json::to_string(&view)?;
    assert_eq!(json, r#"{"id":"0197c0b0-1234-7000-8000-000000000001"}"#);
    let decoded: crate::response::session::SessionView = serde_json::from_str(&json)?;
    assert_eq!(decoded, view);
    Ok(())
}

#[test]
fn empty_view_serializes_as_empty_object() -> anyhow::Result<()> {
    let json = serde_json::to_string(&crate::response::EmptyView {})?;
    assert_eq!(json, "{}");
    let decoded: crate::response::EmptyView = serde_json::from_str("{}")?;
    assert_eq!(decoded, crate::response::EmptyView {});
    Ok(())
}

#[test]
fn comment_view_serializes_parent_id_as_null_when_top_level() -> anyhow::Result<()> {
    let view = crate::response::comment::CommentView {
        id: "c".to_string(),
        content: "hello".to_string(),
        user_id: "u".to_string(),
        parent_id: None,
        created_at: 1,
        user_name: "alice".to_string(),
        child_count: 0,
    };
    let json = serde_json::to_string(&view)?;
    assert!(json.contains(r#""parent_id":null"#));
    let decoded: crate::response::comment::CommentView = serde_json::from_str(&json)?;
    assert_eq!(decoded.parent_id, None);
    Ok(())
}

#[test]
fn email_subject_view_serializes_its_single_field() -> anyhow::Result<()> {
    let view = crate::response::email::EmailSubjectView {
        email_subject: "abc".to_string(),
    };
    let json = serde_json::to_string(&view)?;
    assert_eq!(json, r#"{"email_subject":"abc"}"#);
    let decoded: crate::response::email::EmailSubjectView = serde_json::from_str(&json)?;
    assert_eq!(decoded, view);
    Ok(())
}

#[test]
fn email_subjects_view_serializes_both_subjects() -> anyhow::Result<()> {
    let view = crate::response::email::EmailSubjectsView {
        old_email_subject: "old".to_string(),
        new_email_subject: "new".to_string(),
    };
    let json = serde_json::to_string(&view)?;
    assert_eq!(
        json,
        r#"{"old_email_subject":"old","new_email_subject":"new"}"#
    );
    let decoded: crate::response::email::EmailSubjectsView = serde_json::from_str(&json)?;
    assert_eq!(decoded, view);
    Ok(())
}
