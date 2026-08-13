use std::fmt;

pub fn validate_ascii_text(
    raw: &str,
    max_chars: usize,
    allow_newline: bool,
) -> Result<String, TextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(TextError::Empty);
    }
    for ch in trimmed.chars() {
        if !ch.is_ascii() {
            return Err(TextError::ContainsForbiddenChar(ch));
        }
        let byte = ch as u8;
        if !((0x20..=0x7e).contains(&byte) || (allow_newline && byte == b'\n')) {
            return Err(TextError::ContainsForbiddenChar(ch));
        }
    }
    if trimmed.chars().count() > max_chars {
        return Err(TextError::TooLong { max_chars });
    }
    Ok(trimmed.to_string())
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
                write!(f, "text too long (max {} ascii chars)", max_chars)
            }
            TextError::ContainsForbiddenChar(ch) => {
                write!(
                    f,
                    "text can only contain printable ASCII; forbidden: {:?}",
                    ch
                )
            }
        }
    }
}

impl std::error::Error for TextError {}

#[cfg(test)]
#[path = "../../../test/unit/common/text/tests.rs"]
mod tests;
