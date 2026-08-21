use crate::page::article::version::comment::url::{
    CommentLevel, comment_id_from_level, comment_level_from_path,
};

const COMMENT_ID: &str = "01a018c7-f177-7da1-a821-5f1000648383";

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
    let level = comment_level_from_path(&format!("comment/{COMMENT_ID}"));
    assert!(matches!(level, CommentLevel::Comment(ref id) if id == COMMENT_ID));
}

#[test]
fn comment_delete_suffix_is_the_delete_level() {
    let level = comment_level_from_path(&format!("comment/{COMMENT_ID}/delete"));
    assert!(matches!(level, CommentLevel::DeleteComment(ref id) if id == COMMENT_ID));
}

#[test]
fn comment_update_suffix_is_the_update_level() {
    let level = comment_level_from_path(&format!("comment/{COMMENT_ID}/update"));
    assert!(matches!(
        level,
        CommentLevel::UpdateComment(ref id) if id == COMMENT_ID
    ));
}

#[test]
fn comment_undelete_suffix_is_the_undelete_level() {
    let level = comment_level_from_path(&format!("comment/{COMMENT_ID}/undelete-soft"));
    assert!(matches!(
        level,
        CommentLevel::UndeleteComment(ref id) if id == COMMENT_ID
    ));
}

#[test]
fn malformed_comment_id_is_invalid() {
    assert!(matches!(
        comment_level_from_path("comment/abc123"),
        CommentLevel::Invalid
    ));
    assert!(matches!(
        comment_level_from_path("comment/abc123/delete"),
        CommentLevel::Invalid
    ));
}

#[test]
fn delete_suffix_without_id_is_invalid() {
    assert!(matches!(
        comment_level_from_path("comment//delete"),
        CommentLevel::Invalid
    ));
}

#[test]
fn update_suffix_without_id_is_invalid() {
    assert!(matches!(
        comment_level_from_path("comment//update"),
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
fn id_extraction_matches_all_comment_levels() {
    let comment = CommentLevel::Comment("c1".to_string());
    let update = CommentLevel::UpdateComment("c2".to_string());
    let delete = CommentLevel::DeleteComment("c3".to_string());
    let undelete = CommentLevel::UndeleteComment("c4".to_string());
    assert_eq!(comment_id_from_level(&comment), Some("c1"));
    assert_eq!(comment_id_from_level(&update), Some("c2"));
    assert_eq!(comment_id_from_level(&delete), Some("c3"));
    assert_eq!(comment_id_from_level(&undelete), Some("c4"));
    assert_eq!(comment_id_from_level(&CommentLevel::VersionComments), None);
    assert_eq!(comment_id_from_level(&CommentLevel::Invalid), None);
}
