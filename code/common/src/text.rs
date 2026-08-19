use crate::validate::PrintableAscii;
use crate::validate::ValidationError;
use crate::validate::validate_with_policy;
use std::fmt;

/// Validates and trims printable-ASCII text.
///
/// # Errors
/// Returns [`TextError::Empty`] if blank, [`TextError::TooLong`] if it exceeds
/// `max_chars`, or [`TextError::ContainsForbiddenChar`] for a forbidden character.
pub fn validate_ascii_text(
    raw: &str,
    max_chars: usize,
    allow_newline: bool,
) -> Result<String, TextError> {
    validate_with_policy::<TextError, _>(raw, max_chars, &PrintableAscii { allow_newline })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextError {
    Empty,
    TooLong { max_chars: usize },
    ContainsForbiddenChar(char),
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextError::Empty => write!(f, "text cannot be empty"),
            TextError::TooLong { max_chars } => {
                write!(f, "text too long (max {max_chars} ascii chars)")
            }
            TextError::ContainsForbiddenChar(ch) => {
                write!(
                    f,
                    "text can only contain printable ASCII; forbidden: {ch:?}"
                )
            }
        }
    }
}

impl std::error::Error for TextError {}

impl ValidationError for TextError {
    fn empty() -> Self {
        TextError::Empty
    }
    fn too_long(max_chars: usize) -> Self {
        TextError::TooLong { max_chars }
    }
    fn forbidden(ch: char) -> Self {
        TextError::ContainsForbiddenChar(ch)
    }
}

#[cfg(test)]
#[path = "../../../test/unit/common/text/tests.rs"]
mod tests;
