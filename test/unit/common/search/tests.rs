use crate::search::SearchRange;

#[test]
fn search_range_serializes_as_lowercase_strings() -> anyhow::Result<()> {
    for (range, expected) in [
        (SearchRange::Title, "title"),
        (SearchRange::Summary, "summary"),
        (SearchRange::AuthorName, "author_name"),
        (SearchRange::Comment, "comment"),
        (SearchRange::Note, "note"),
        (SearchRange::Tag, "tag"),
        (SearchRange::VersionNumber, "version_number"),
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
    assert_eq!(
        serde_json::from_str::<SearchRange>(r#""author_name""#)?,
        SearchRange::AuthorName
    );
    assert_eq!(
        serde_json::from_str::<SearchRange>(r#""version_number""#)?,
        SearchRange::VersionNumber
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
        (SearchRange::Title, "title"),
        (SearchRange::Summary, "summary"),
        (SearchRange::AuthorName, "author"),
        (SearchRange::Comment, "comment"),
        (SearchRange::Note, "note"),
        (SearchRange::Tag, "tag"),
        (SearchRange::VersionNumber, "version"),
    ];
    for (range, label) in expected {
        assert_eq!(range.label(), label);
    }
}
