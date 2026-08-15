pub enum CommentLevel {
    VersionComments,
    Comment(String),
    DeleteComment(String),
    Invalid,
}

pub fn comment_level_from_path(comment_path: &str) -> CommentLevel {
    if comment_path == "comment" {
        return CommentLevel::VersionComments;
    }
    if let Some(rest) = comment_path.strip_prefix("comment/") {
        if let Some(comment_id) = rest.strip_suffix("/delete") {
            if !comment_id.is_empty() {
                return CommentLevel::DeleteComment(comment_id.to_string());
            }
        } else if !rest.is_empty() {
            return CommentLevel::Comment(rest.to_string());
        }
    }
    CommentLevel::Invalid
}

pub fn comment_id_from_level(level: &CommentLevel) -> Option<&str> {
    match level {
        CommentLevel::Comment(comment_id) | CommentLevel::DeleteComment(comment_id) => {
            Some(comment_id)
        }
        CommentLevel::VersionComments | CommentLevel::Invalid => None,
    }
}

#[cfg(test)]
#[path = "../../../../../../../../test/unit/front/page/comment/url/tests.rs"]
mod tests;
