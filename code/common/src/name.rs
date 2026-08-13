use std::fmt;

pub const MAX_NAME_CHAR_COUNT: usize = 32;

pub fn validate_name(raw_name: &str) -> Result<String, NameError> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return Err(NameError::Empty);
    }
    for ch in trimmed.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
            return Err(NameError::ContainsForbiddenChar(ch));
        }
    }
    if trimmed.chars().count() > MAX_NAME_CHAR_COUNT {
        return Err(NameError::TooLong);
    }
    Ok(trimmed.to_string())
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
            NameError::TooLong => write!(
                f,
                "name too long (max {} unicode chars)",
                MAX_NAME_CHAR_COUNT
            ),
            NameError::ContainsForbiddenChar(ch) => write!(f, "name cannot contain {:?}", ch),
        }
    }
}

impl std::error::Error for NameError {}

#[cfg(test)]
#[path = "../../../test/unit/common/name/tests.rs"]
mod tests;
