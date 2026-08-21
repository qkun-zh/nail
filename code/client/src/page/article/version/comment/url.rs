use crate::page::validation::validate_uuid;

pub enum CommentLevel {
    VersionComments,
    Comment(String),
    UpdateComment(String),
    DeleteComment(String),
    UndeleteComment(String),
    Invalid,
}

pub fn comment_level_from_path(comment_path: &str) -> CommentLevel {
    if comment_path == "comment" {
        return CommentLevel::VersionComments;
    }
    if let Some(rest) = comment_path.strip_prefix("comment/") {
        if let Some(comment_id) = rest.strip_suffix("/delete") {
            return valid_comment_level(comment_id, CommentLevel::DeleteComment);
        } else if let Some(comment_id) = rest.strip_suffix("/update") {
            return valid_comment_level(comment_id, CommentLevel::UpdateComment);
        } else if let Some(comment_id) = rest.strip_suffix("/undelete-soft") {
            return valid_comment_level(comment_id, CommentLevel::UndeleteComment);
        } else if !rest.is_empty() {
            return valid_comment_level(rest, CommentLevel::Comment);
        }
    }
    CommentLevel::Invalid
}

fn valid_comment_level(
    comment_id: &str,
    level: impl FnOnce(String) -> CommentLevel,
) -> CommentLevel {
    match validate_uuid(comment_id) {
        Ok(value) => level(value),
        Err(_) => CommentLevel::Invalid,
    }
}

pub fn comment_id_from_level(level: &CommentLevel) -> Option<&str> {
    match level {
        CommentLevel::Comment(comment_id)
        | CommentLevel::UpdateComment(comment_id)
        | CommentLevel::DeleteComment(comment_id)
        | CommentLevel::UndeleteComment(comment_id) => Some(comment_id),
        CommentLevel::VersionComments | CommentLevel::Invalid => None,
    }
}

#[cfg(test)]
#[path = "../../../../../../../test/unit/client/page/comment/url/tests.rs"]
mod tests;
