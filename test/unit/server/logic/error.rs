use axum::http::StatusCode;

use crate::logic::error::LogicError;
use crate::repository::article::{CreateArticleError, UpdateArticleError};
use crate::repository::authorization::AssemblyError;
use crate::repository::comment::CreateCommentError;
use crate::repository::transfer::{TransferError, TransferTargetError};
use crate::repository::user::UserWriteError;
use crate::repository::version::CreateVersionError;

fn db_error() -> database::Error {
    database::Error::Storage("boom".to_string())
}

#[test]
fn from_db_error_maps_to_internal_database_query_failed() {
    let error = db_error();
    let expected = LogicError::internal(format!("database query failed: {error}"));
    assert_eq!(LogicError::from(error), expected);
}

#[test]
fn from_create_article_error_maps_every_variant() {
    let db = db_error();
    let expected_db = LogicError::internal(format!("database query failed: {db}"));
    assert_eq!(
        LogicError::from(CreateArticleError::AuthorMissing),
        LogicError::internal("author not found")
    );
    assert_eq!(
        LogicError::from(CreateArticleError::TitleTaken),
        LogicError::bad_request("title already exists")
    );
    assert_eq!(
        LogicError::from(CreateArticleError::ContentHashTaken),
        LogicError::bad_request("identical PDF already exists")
    );
    assert_eq!(LogicError::from(CreateArticleError::Db(db)), expected_db);
}

#[test]
fn from_update_article_error_maps_every_variant() {
    let db = db_error();
    let expected_db = LogicError::internal(format!("database query failed: {db}"));
    assert_eq!(
        LogicError::from(UpdateArticleError::Missing),
        LogicError::not_found("article not found")
    );
    assert_eq!(
        LogicError::from(UpdateArticleError::TitleTaken),
        LogicError::bad_request("title already exists")
    );
    assert_eq!(LogicError::from(UpdateArticleError::Db(db)), expected_db);
}

#[test]
fn from_create_comment_error_maps_every_variant() {
    let db = db_error();
    let expected_db = LogicError::internal(format!("database query failed: {db}"));
    assert_eq!(
        LogicError::from(CreateCommentError::TargetNotFound),
        LogicError::not_found("comment target not found (the version may have been removed)")
    );
    assert_eq!(
        LogicError::from(CreateCommentError::CommentIdExists),
        LogicError::internal("comment id already exists")
    );
    assert_eq!(
        LogicError::from(CreateCommentError::CommentTreeTooDeep),
        LogicError::bad_request("comment thread too deep (max 64 reply layers)")
    );
    assert_eq!(LogicError::from(CreateCommentError::Db(db)), expected_db);
}

#[test]
fn from_create_version_error_maps_every_variant() {
    let db = db_error();
    let expected_db = LogicError::internal(format!("database query failed: {db}"));
    assert_eq!(
        LogicError::from(CreateVersionError::ArticleMissing),
        LogicError::not_found("article not found")
    );
    assert_eq!(
        LogicError::from(CreateVersionError::NotGreater),
        LogicError::bad_request("new version must be strictly greater than the latest version")
    );
    assert_eq!(
        LogicError::from(CreateVersionError::InvalidNumber),
        LogicError::bad_request("invalid version number")
    );
    assert_eq!(
        LogicError::from(CreateVersionError::ContentHashTaken),
        LogicError::bad_request("identical PDF already exists")
    );
    assert_eq!(LogicError::from(CreateVersionError::Db(db)), expected_db);
}

#[test]
fn from_transfer_error_maps_every_variant() {
    let db = db_error();
    let expected_db = LogicError::internal(format!("failed to transfer account assets: {db}"));
    assert_eq!(
        LogicError::from(TransferError::NoRecycler),
        LogicError::internal("no recycler available")
    );
    assert_eq!(LogicError::from(TransferError::Db(db)), expected_db);
}

#[test]
fn from_transfer_target_error_maps_every_variant() {
    let db = db_error();
    let expected_db = LogicError::internal(format!("database query failed: {db}"));
    assert_eq!(
        LogicError::from(TransferTargetError::TargetMissing),
        LogicError::not_found("article not found")
    );
    assert_eq!(
        LogicError::from(TransferTargetError::TargetOwnerMissing),
        LogicError::internal("article has no owner")
    );
    assert_eq!(
        LogicError::from(TransferTargetError::NoRecycler),
        LogicError::internal("no recycler available")
    );
    assert_eq!(LogicError::from(TransferTargetError::Db(db)), expected_db);
}

#[test]
fn from_user_write_error_maps_every_variant() {
    let db = db_error();
    let expected_db = LogicError::internal(format!("failed to update name: {db}"));
    assert_eq!(
        LogicError::from(UserWriteError::AlreadyTaken),
        LogicError::bad_request("name already taken")
    );
    assert_eq!(
        LogicError::from(UserWriteError::UserMissing),
        LogicError::unauthorized("user not found")
    );
    assert_eq!(
        LogicError::from(UserWriteError::EmailMismatch),
        LogicError::internal("unexpected email mismatch")
    );
    assert_eq!(LogicError::from(UserWriteError::Db(db)), expected_db);
}

#[test]
fn from_assembly_error_maps_every_variant() {
    assert_eq!(
        LogicError::from(AssemblyError::ResourceNotFound),
        LogicError::not_found("resource not found")
    );
    assert_eq!(
        LogicError::from(AssemblyError::Internal("boom".to_string())),
        LogicError::internal("boom")
    );
}

#[test]
fn every_variant_maps_to_its_status_code() {
    let cases = [
        (LogicError::bad_request("bad"), StatusCode::BAD_REQUEST),
        (LogicError::unauthorized("unauth"), StatusCode::UNAUTHORIZED),
        (LogicError::forbidden("denied"), StatusCode::FORBIDDEN),
        (LogicError::not_found("missing"), StatusCode::NOT_FOUND),
        (
            LogicError::internal("boom"),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.status(), expected);
    }
}

#[test]
fn internal_error_is_masked_in_the_envelope_pair() {
    let (status, message) = LogicError::internal("secret database detail").into_pair();
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(message, "internal server error");
}

#[test]
fn non_internal_errors_keep_their_message_in_the_envelope_pair() {
    let (status, message) = LogicError::not_found("article not found").into_pair();
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(message, "article not found");
}

#[test]
fn message_exposes_the_reason_for_every_variant() {
    assert_eq!(LogicError::bad_request("x").message(), "x");
    assert_eq!(LogicError::unauthorized("y").message(), "y");
    assert_eq!(LogicError::forbidden("z").message(), "z");
    assert_eq!(LogicError::not_found("w").message(), "w");
    assert_eq!(LogicError::internal("v").message(), "v");
}

#[test]
fn display_delegates_to_message() {
    let error = LogicError::bad_request("something went wrong");
    assert_eq!(format!("{error}"), "something went wrong");
}

#[test]
fn database_error_wraps_the_cause_as_internal() {
    let error = crate::logic::error::database_error("connection refused");
    assert_eq!(
        error,
        LogicError::internal("database query failed: connection refused")
    );
}
