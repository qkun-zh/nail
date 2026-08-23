use std::fmt;

use leptos::prelude::*;

use crate::page::notify::{notify_error, use_notifications};
use crate::page::validation::validate_uuid;
use crate::request::error::RequestError;

/// Outcome of a page-level data load; `Err` carries a display-ready message.
pub type Loaded<T> = Result<T, LoadError>;

/// Display-ready load failure unifying validation messages and request errors.
#[derive(Debug, Clone)]
pub struct LoadError(String);

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<String> for LoadError {
    fn from(message: String) -> Self {
        Self(message)
    }
}

impl From<RequestError> for LoadError {
    fn from(error: RequestError) -> Self {
        Self(error.to_string())
    }
}

/// Validate a path parameter as a UUID under the unified load-error type.
pub fn require_id(raw: &str) -> Result<String, LoadError> {
    validate_uuid(raw).map_err(LoadError::from)
}

/// Toast failed loads while keeping them available inline through the resource.
pub fn notify_load_failures<T: Clone + 'static>(resource: LocalResource<Loaded<T>>) {
    let notifications = use_notifications();
    Effect::new(move |_| {
        if let Some(Err(message)) = resource.get() {
            notify_error(&notifications, message.to_string());
        }
    });
}
