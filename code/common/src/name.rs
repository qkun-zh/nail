use crate::validate::AlphanumericDashUnderscore;
use crate::validate::ValidationError;
use crate::validate::validate_with_policy;
use std::fmt;

pub const MAX_NAME_CHAR_COUNT: usize = 32;

pub fn validate_name(raw_name: &str) -> Result<String, NameError> {
    validate_with_policy::<NameError, _>(raw_name, MAX_NAME_CHAR_COUNT, &AlphanumericDashUnderscore)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong,
    ContainsForbiddenChar(char),
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameError::Empty => write!(f, "name cannot be empty"),
            NameError::TooLong => {
                write!(f, "name too long (max {MAX_NAME_CHAR_COUNT} unicode chars)")
            }
            NameError::ContainsForbiddenChar(ch) => write!(f, "name cannot contain {ch:?}"),
        }
    }
}

impl std::error::Error for NameError {}

impl ValidationError for NameError {
    fn empty() -> Self {
        NameError::Empty
    }
    fn too_long(_max_chars: usize) -> Self {
        NameError::TooLong
    }
    fn forbidden(ch: char) -> Self {
        NameError::ContainsForbiddenChar(ch)
    }
}

#[cfg(test)]
#[path = "name_tests.rs"]
mod tests;
