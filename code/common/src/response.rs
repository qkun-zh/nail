use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseEnvelope<T> {
    pub code: u16,
    pub data: Option<T>,
    pub message: String,
}

impl<T> ResponseEnvelope<T> {
    pub fn ok(code: u16, data: T, message: impl Into<String>) -> Self {
        Self {
            code,
            data: Some(data),
            message: message.into(),
        }
    }

    pub fn err(code: u16, message: impl Into<String>) -> Self {
        Self {
            code,
            data: None,
            message: message.into(),
        }
    }
}

#[cfg(test)]
#[path = "../../../test/unit/common/response/tests.rs"]
mod tests;
