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
        (SearchRange::ArticleId, "article_id"),
        (SearchRange::VersionId, "version_id"),
        (SearchRange::CommentId, "comment_id"),
        (SearchRange::AuthorId, "author_id"),
        (SearchRange::Role, "role"),
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
    assert_eq!(
        serde_json::from_str::<SearchRange>(r#""article_id""#)?,
        SearchRange::ArticleId
    );
    assert_eq!(
        serde_json::from_str::<SearchRange>(r#""version_id""#)?,
        SearchRange::VersionId
    );
    assert_eq!(
        serde_json::from_str::<SearchRange>(r#""comment_id""#)?,
        SearchRange::CommentId
    );
    assert_eq!(
        serde_json::from_str::<SearchRange>(r#""author_id""#)?,
        SearchRange::AuthorId
    );
    assert_eq!(
        serde_json::from_str::<SearchRange>(r#""role""#)?,
        SearchRange::Role
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
        (SearchRange::ArticleId, "article id"),
        (SearchRange::VersionId, "version id"),
        (SearchRange::CommentId, "comment id"),
        (SearchRange::AuthorId, "author id"),
        (SearchRange::Role, "role"),
    ];
    for (range, label) in expected {
        assert_eq!(range.label(), label);
    }
}

#[test]
fn search_range_as_str_matches_wire_and_round_trips() -> anyhow::Result<()> {
    for range in [
        SearchRange::Title,
        SearchRange::Summary,
        SearchRange::AuthorName,
        SearchRange::Comment,
        SearchRange::Note,
        SearchRange::Tag,
        SearchRange::VersionNumber,
        SearchRange::ArticleId,
        SearchRange::VersionId,
        SearchRange::CommentId,
        SearchRange::AuthorId,
        SearchRange::Role,
    ] {
        let wire = serde_json::to_string(&range)?;
        assert_eq!(wire, format!("\"{}\"", range.as_str()));
        let parsed: SearchRange = range
            .as_str()
            .parse()
            .map_err(|message: String| anyhow::anyhow!("{message}"))?;
        assert_eq!(parsed, range);
    }
    Ok(())
}
