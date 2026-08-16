use serde::{Deserialize, Serialize};
use std::fmt;

pub const MAX_TAG_NAME_CHAR_COUNT: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagRef {
    pub id: String,
    pub name: String,
}

/// Validates and trims a single tag name.
///
/// # Errors
/// Returns [`TagNameError::Empty`] if blank, [`TagNameError::TooLong`] if it
/// exceeds [`MAX_TAG_NAME_CHAR_COUNT`], or
/// [`TagNameError::ContainsForbiddenChar`] for an invalid character.
pub fn validate_tag_name(raw_name: &str) -> Result<String, TagNameError> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return Err(TagNameError::Empty);
    }
    for ch in trimmed.chars() {
        if !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
            return Err(TagNameError::ContainsForbiddenChar(ch));
        }
    }
    if trimmed.chars().count() > MAX_TAG_NAME_CHAR_COUNT {
        return Err(TagNameError::TooLong);
    }
    Ok(trimmed.to_string())
}

/// Parses whitespace-separated tags, deduplicating them.
///
/// # Errors
/// Returns [`TagNamesError::Name`] for an invalid tag or
/// [`TagNamesError::TooManyTags`] when more than `max_count` tags are given.
pub fn parse_tags(raw: &str, max_count: usize) -> Result<Vec<String>, TagNamesError> {
    let mut seen = std::collections::HashSet::new();
    let mut tags = Vec::new();
    for piece in raw.split_whitespace() {
        let name = validate_tag_name(piece).map_err(TagNamesError::Name)?;
        if seen.insert(name.clone()) {
            tags.push(name);
        }
        if tags.len() > max_count {
            return Err(TagNamesError::TooManyTags { max_count });
        }
    }
    Ok(tags)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagNameError {
    Empty,
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
            TagNameError::Empty => write!(f, "tag name cannot be empty"),
            TagNameError::TooLong => {
                write!(f, "tag name too long (max {MAX_TAG_NAME_CHAR_COUNT} chars)")
            }
            TagNameError::ContainsForbiddenChar(ch) => {
                write!(f, "tag name cannot contain '{ch}'")
            }
        }
    }
}

impl fmt::Display for TagNamesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TagNamesError::Name(name_error) => write!(f, "{name_error}"),
            TagNamesError::TooManyTags { max_count } => {
                write!(f, "too many tags (max {max_count})")
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../test/unit/common/tag/tests.rs"]
mod tests;
