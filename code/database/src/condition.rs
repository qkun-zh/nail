/// Filter pushed down to storage scans.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    KeyEquals(String, crate::Value),
    KeyGreaterThan(String, crate::Value),
    KeyNotExists(String),
    All(Vec<Condition>),
}

/// Deterministic ordering for scans and pagination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub key: String,
    pub ascending: bool,
}

impl Order {
    #[must_use]
    pub fn ascending(key: &str) -> Self {
        Self {
            key: key.to_string(),
            ascending: true,
        }
    }

    #[must_use]
    pub fn descending(key: &str) -> Self {
        Self {
            key: key.to_string(),
            ascending: false,
        }
    }
}
