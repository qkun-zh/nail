use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestErrorKind {
    Network,
    Status,
    EmptyData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestError {
    pub kind: RequestErrorKind,
    pub message: String,
}

impl RequestError {
    pub fn network(message: impl Into<String>) -> Self {
        Self {
            kind: RequestErrorKind::Network,
            message: message.into(),
        }
    }

    pub fn status(code: u16, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            kind: RequestErrorKind::Status,
            message: format!("[HTTP {code}] {message}"),
        }
    }

    pub fn empty_data() -> Self {
        Self {
            kind: RequestErrorKind::EmptyData,
            message: "the server returned an empty payload".to_string(),
        }
    }
}

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RequestError {}

pub type RequestResult<T> = Result<T, RequestError>;
