use serde::{Deserialize, Serialize};
use std::fmt;

pub const MAX_TAG_NAME_CHAR_COUNT: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagRef {
    pub id: String,
    pub name: String,
}

pub fn validate_tag_name(raw_name: &str) -> Result<String, TagNameError> {
    let trimmed = raw_name.trim();
    if !trimmed.starts_with('#') {
        return Err(TagNameError::MissingHash);
    }
    if trimmed.chars().count() < 2 {
        return Err(TagNameError::Empty);
    }
    for ch in trimmed.chars().skip(1) {
        if !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
            return Err(TagNameError::ContainsForbiddenChar(ch));
        }
    }
    if trimmed.chars().count() > MAX_TAG_NAME_CHAR_COUNT {
        return Err(TagNameError::TooLong);
    }
    Ok(trimmed.to_string())
}

pub fn parse_hashtag_tags(raw: &str, max_count: usize) -> Result<Vec<String>, TagNamesError> {
    let mut seen = std::collections::HashSet::new();
    let mut tags = Vec::new();
    for piece in raw.split_whitespace() {
        if !piece.starts_with('#') {
            return Err(TagNamesError::Name(TagNameError::MissingHash));
        }
        let mut segments = piece.split('#');
        let _leading = segments.next();
        for segment in segments {
            if segment.is_empty() {
                return Err(TagNamesError::Name(TagNameError::Empty));
            }
            let name = format!("#{segment}");
            let name = validate_tag_name(&name).map_err(TagNamesError::Name)?;
            if seen.insert(name.clone()) {
                tags.push(name);
            }
            if tags.len() > max_count {
                return Err(TagNamesError::TooManyTags { max_count });
            }
        }
    }
    Ok(tags)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagNameError {
    Empty,
    MissingHash,
    TooLong,
    ContainsForbiddenChar(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagNamesError {
    Name(TagNameError),
    TooManyTags { max_count: usize },
}

impl fmt::Display for TagNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TagNameError::Empty => write!(
                f,
                "tag name cannot be empty (must be '#' plus at least 1 char)"
            ),
            TagNameError::MissingHash => write!(f, "tag name must start with '#'"),
            TagNameError::TooLong => write!(
                f,
                "tag name too long (max {} chars including '#')",
                MAX_TAG_NAME_CHAR_COUNT
            ),
            TagNameError::ContainsForbiddenChar(ch) => {
                write!(f, "tag name cannot contain '{}' after '#'", ch)
            }
        }
    }
}

impl fmt::Display for TagNamesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TagNamesError::Name(name_error) => write!(f, "{}", name_error),
            TagNamesError::TooManyTags { max_count } => {
                write!(f, "too many tags (max {})", max_count)
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../test/unit/common/tag/tests.rs"]
mod tests;
