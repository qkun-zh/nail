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
    assert_eq!(
        serde_json::from_str::<SearchRange>(r#""title""#)?,
        SearchRange::Title
    );
    assert_eq!(
        serde_json::from_str::<SearchRange>(r#""tag""#)?,
        SearchRange::Tag
    );
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
    assert_eq!(
        serde_json::to_string(&SearchSortField::Title)?,
        r#""title""#
    );
    assert_eq!(
        serde_json::to_string(&SearchSortField::Author)?,
        r#""author""#
    );
    assert_eq!(
        serde_json::to_string(&SearchSortDirection::Asc)?,
        r#""asc""#
    );
    assert_eq!(
        serde_json::to_string(&SearchSortDirection::Desc)?,
        r#""desc""#
    );
    Ok(())
}
