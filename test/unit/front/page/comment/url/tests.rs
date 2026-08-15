use crate::page::public::article::version::comment::url::{
    CommentLevel, comment_id_from_level, comment_level_from_path,
};

#[test]
fn empty_path_is_invalid() {
    assert!(matches!(comment_level_from_path(""), CommentLevel::Invalid));
}

#[test]
fn comment_root_is_the_version_comments_level() {
    assert!(matches!(
        comment_level_from_path("comment"),
        CommentLevel::VersionComments
    ));
}

#[test]
fn comment_with_id_is_the_comment_level() {
    let level = comment_level_from_path("comment/abc123");
    assert!(matches!(level, CommentLevel::Comment(ref id) if id == "abc123"));
}

#[test]
fn comment_delete_suffix_is_the_delete_level() {
    let level = comment_level_from_path("comment/abc123/delete");
    assert!(matches!(level, CommentLevel::DeleteComment(ref id) if id == "abc123"));
}

#[test]
fn delete_suffix_without_id_is_invalid() {
    assert!(matches!(
        comment_level_from_path("comment//delete"),
        CommentLevel::Invalid
    ));
}

#[test]
fn unrelated_prefix_is_invalid() {
    assert!(matches!(
        comment_level_from_path("something/else"),
        CommentLevel::Invalid
    ));
}

#[test]
fn id_extraction_matches_comment_and_delete_levels() {
    let comment = CommentLevel::Comment("c1".to_string());
    let delete = CommentLevel::DeleteComment("c2".to_string());
    assert_eq!(comment_id_from_level(&comment), Some("c1"));
    assert_eq!(comment_id_from_level(&delete), Some("c2"));
    assert_eq!(comment_id_from_level(&CommentLevel::VersionComments), None);
    assert_eq!(comment_id_from_level(&CommentLevel::Invalid), None);
}
