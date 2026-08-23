use crate::validate::AlphanumericDashUnderscore;
use crate::validate::ValidationError;
use crate::validate::validate_with_policy;
use std::fmt;

pub const MAX_TAG_NAME_CHAR_COUNT: usize = 32;

pub fn validate_tag_name(raw_name: &str) -> Result<String, TagNameError> {
    validate_with_policy::<TagNameError, _>(
        raw_name,
        MAX_TAG_NAME_CHAR_COUNT,
        &AlphanumericDashUnderscore,
    )
}

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

impl ValidationError for TagNameError {
    fn empty() -> Self {
        TagNameError::Empty
    }
    fn too_long(_max_chars: usize) -> Self {
        TagNameError::TooLong
    }
    fn forbidden(ch: char) -> Self {
        TagNameError::ContainsForbiddenChar(ch)
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
#[path = "tag_tests.rs"]
mod tests;
