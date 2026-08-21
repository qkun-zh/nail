use common::response::ResponseEnvelope;
use serde::de::DeserializeOwned;

use crate::request::error::{RequestError, RequestResult};

pub fn is_success(code: u16) -> bool {
    (200..300).contains(&code)
}

pub fn parse_envelope<T: DeserializeOwned>(text: &str) -> RequestResult<ResponseEnvelope<T>> {
    serde_json::from_str(text).map_err(|error| {
        RequestError::network(format!("failed to parse response envelope: {error}"))
    })
}

pub fn unwrap_envelope<T>(envelope: ResponseEnvelope<T>) -> RequestResult<T> {
    if !is_success(envelope.code) {
        return Err(RequestError::status(envelope.code, envelope.message));
    }
    envelope.data.ok_or_else(RequestError::empty_data)
}

#[cfg(test)]
#[path = "envelope_tests.rs"]
mod tests;
