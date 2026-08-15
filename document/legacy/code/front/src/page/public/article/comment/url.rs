
pub enum CommentLevel {
    VersionPage,
    VersionComments,
    Comment(String),
    DeleteComment(String),
    Invalid,
}

fn is_valid_comment_id_segment(cid: &str) -> bool {
    !cid.is_empty() && cid.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

pub fn comment_level_id(comment_path: &str) -> Option<String> {
    comment_path
        .strip_prefix("comment/")
        .filter(|cid| is_valid_comment_id_segment(cid))
        .map(|cid| cid.to_string())
}

pub fn comment_delete_id(comment_path: &str) -> Option<String> {
    comment_path
        .strip_prefix("comment/")
        .and_then(|rest| rest.strip_suffix("/delete"))
        .filter(|cid| is_valid_comment_id_segment(cid))
        .map(|cid| cid.to_string())
}

pub fn comment_level_from_path(comment_path: &str) -> CommentLevel {
    if comment_path.is_empty() {
        return CommentLevel::VersionPage;
    }
    if comment_path == "comment" {
        return CommentLevel::VersionComments;
    }
    if let Some(cid) = comment_delete_id(comment_path) {
        return CommentLevel::DeleteComment(cid);
    }
    if let Some(cid) = comment_level_id(comment_path) {
        return CommentLevel::Comment(cid);
    }
    CommentLevel::Invalid
}
