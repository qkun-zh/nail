use uuid::Uuid;

use crate::request::error::{RequestError, RequestResult};

pub fn validate_id(raw: &str, label: &str) -> RequestResult<String> {
    let value = raw.trim();
    Uuid::parse_str(value)
        .map(|uuid| uuid.to_string())
        .map_err(|_| RequestError::status(400, format!("invalid {label}: expected a UUID")))
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;
