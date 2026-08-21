use crate::field::SearchField;

#[test]
fn search_fields_map_to_exact_engine_names() {
    let expected = [
        (SearchField::Title, "title"),
        (SearchField::Summary, "summary"),
        (SearchField::AuthorName, "author_name"),
        (SearchField::Comment, "content"),
        (SearchField::Note, "note"),
        (SearchField::Tag, "tags"),
        (SearchField::VersionNumber, "version_number"),
        (SearchField::ArticleId, "article_id"),
        (SearchField::VersionId, "version_id"),
        (SearchField::CommentId, "comment_id"),
        (SearchField::AuthorId, "author_id"),
        (SearchField::Role, "role"),
    ];
    for (field, name) in expected {
        assert_eq!(field.as_engine_field(), name);
    }
}
