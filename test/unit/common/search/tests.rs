use crate::search::SearchRange;
use crate::search::SearchSortDirection;
use crate::search::SearchSortField;

#[test]
fn search_range_serializes_as_lowercase_strings() -> anyhow::Result<()> {
    for (range, expected) in [
        (SearchRange::Title, "title"),
        (SearchRange::Summary, "summary"),
        (SearchRange::Author, "author"),
        (SearchRange::Comment, "comment"),
        (SearchRange::Note, "note"),
        (SearchRange::Tag, "tag"),
    ] {
        assert_eq!(serde_json::to_string(&range)?, format!("\"{expected}\""));
    }
    Ok(())
}

#[test]
fn search_range_deserializes_from_lowercase_strings() -> anyhow::Result<()> {
    assert_eq!(serde_json::from_str::<SearchRange>(r#""title""#)?, SearchRange::Title);
    assert_eq!(serde_json::from_str::<SearchRange>(r#""tag""#)?, SearchRange::Tag);
    Ok(())
}

#[test]
fn search_range_rejects_unknown_values() {
    for value in [r#""Title""#, r#""body""#, r#""""#] {
        let result = serde_json::from_str::<SearchRange>(value);
        assert!(result.is_err(), "value {value} must be rejected");
    }
}

#[test]
fn search_range_labels_are_english() {
    let expected = [
        (SearchRange::Title, "Title"),
        (SearchRange::Summary, "Summary"),
        (SearchRange::Author, "Author"),
        (SearchRange::Comment, "Comment"),
        (SearchRange::Note, "Version note"),
        (SearchRange::Tag, "Tag"),
    ];
    for (range, label) in expected {
        assert_eq!(range.label(), label);
    }
}

#[test]
fn search_sort_enums_serialize_as_lowercase_strings() -> anyhow::Result<()> {
    assert_eq!(serde_json::to_string(&SearchSortField::Time)?, r#""time""#);
    assert_eq!(serde_json::to_string(&SearchSortField::Title)?, r#""title""#);
    assert_eq!(serde_json::to_string(&SearchSortField::Author)?, r#""author""#);
    assert_eq!(serde_json::to_string(&SearchSortDirection::Asc)?, r#""asc""#);
    assert_eq!(serde_json::to_string(&SearchSortDirection::Desc)?, r#""desc""#);
    Ok(())
}

#[test]
fn search_hit_round_trips_with_lowercase_field() -> anyhow::Result<()> {
    let hit = crate::search::SearchHit {
        field: SearchRange::Title,
        label: "Title".to_string(),
        snippet: "Rust <mark>borrow</mark> checker".to_string(),
    };
    let json = serde_json::to_string(&hit)?;
    assert_eq!(
        json,
        r##"{"field":"title","label":"Title","snippet":"Rust <mark>borrow</mark> checker"}"##
    );
    let decoded: crate::search::SearchHit = serde_json::from_str(&json)?;
    assert_eq!(decoded, hit);
    Ok(())
}

#[test]
fn search_article_item_round_trips_with_hits() -> anyhow::Result<()> {
    let item = crate::search::SearchArticleItem {
        id: "0197c0b0-1234-7000-8000-000000000001".to_string(),
        title: "Rust borrow checker".to_string(),
        author: "alice".to_string(),
        time: "2023-11-15T06:13:20+08:00".to_string(),
        hits: vec![crate::search::SearchHit {
            field: SearchRange::Summary,
            label: "Summary".to_string(),
            snippet: "A <mark>summary</mark>".to_string(),
        }],
    };
    let json = serde_json::to_string(&item)?;
    let decoded: crate::search::SearchArticleItem = serde_json::from_str(&json)?;
    assert_eq!(decoded, item);
    Ok(())
}

#[test]
fn search_page_round_trips_with_paging_fields() -> anyhow::Result<()> {
    let page = crate::search::SearchPage {
        article_list: Vec::new(),
        total: 0,
        page: 1,
        total_pages: 1,
        has_more: false,
        has_prev: false,
        truncated: false,
    };
    let json = serde_json::to_string(&page)?;
    let decoded: crate::search::SearchPage = serde_json::from_str(&json)?;
    assert_eq!(decoded, page);
    Ok(())
}

#[test]
fn article_search_params_round_trip_all_fields() -> anyhow::Result<()> {
    let params = crate::search::ArticleSearchParams {
        q: Some("rust".to_string()),
        ranges: Some("title,author".to_string()),
        sort: Some("time:desc".to_string()),
        from: Some(1_700_000_000),
        to: Some(1_700_100_000),
        limit: Some(8),
        page: Some(1),
    };
    let json = serde_json::to_string(&params)?;
    let decoded: crate::search::ArticleSearchParams = serde_json::from_str(&json)?;
    assert_eq!(decoded, params);
    Ok(())
}

#[test]
fn article_search_params_default_to_all_none() -> anyhow::Result<()> {
    let decoded: crate::search::ArticleSearchParams = serde_json::from_str("{}")?;
    assert_eq!(decoded, crate::search::ArticleSearchParams::default());
    Ok(())
}
