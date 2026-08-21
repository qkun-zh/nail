use axum::http::StatusCode;

use crate::interface::envelope::ApiError;
use crate::logic::error::LogicError;

#[test]
fn from_logic_maps_each_logic_error_variant_to_its_status_and_message() {
    let cases = [
        (
            LogicError::bad_request("bad"),
            StatusCode::BAD_REQUEST,
            "bad",
        ),
        (
            LogicError::unauthorized("unauth"),
            StatusCode::UNAUTHORIZED,
            "unauth",
        ),
        (
            LogicError::forbidden("denied"),
            StatusCode::FORBIDDEN,
            "denied",
        ),
        (LogicError::not_found("gone"), StatusCode::NOT_FOUND, "gone"),
        (
            LogicError::internal("boom"),
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error",
        ),
    ];
    for (error, status, message) in cases {
        let converted = ApiError::from_logic(error);
        assert_eq!(converted.status, status);
        assert_eq!(converted.message, message);
    }
}

#[test]
fn with_status_keeps_the_given_status_and_message() {
    let error = ApiError::with_status(StatusCode::PAYLOAD_TOO_LARGE, "too big");
    assert_eq!(error.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(error.message, "too big");
}
