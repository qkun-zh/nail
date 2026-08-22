use axum::http::StatusCode;

use crate::repository::article::{CreateArticleError, UpdateArticleError};
use crate::repository::comment::CreateCommentError;
use crate::repository::transfer::{TransferError, TransferTargetError};
use crate::repository::user::UserWriteError;
use crate::repository::version::CreateVersionError;

pub(crate) const MAX_COMMENT_TREE_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Internal(String),
}

impl LogicError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized(message.into())
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::BadRequest(message)
            | Self::Unauthorized(message)
            | Self::Forbidden(message)
            | Self::NotFound(message)
            | Self::Internal(message) => message,
        }
    }

    pub fn into_pair(self) -> (StatusCode, String) {
        let status = self.status();
        let message = match self {
            Self::Internal(_) => "internal server error".to_string(),
            other => other.message().to_string(),
        };
        (status, message)
    }
}

impl std::fmt::Display for LogicError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for LogicError {}

#[must_use]
pub fn usize_capped(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

pub fn validate_ascii_text_capped(
    raw: &str,
    max_chars: u64,
    allow_newline: bool,
) -> Result<String, LogicError> {
    common::text::validate_ascii_text(raw, usize_capped(max_chars), allow_newline)
        .map_err(|error| LogicError::bad_request(error.to_string()))
}

pub fn database_error(error: impl std::fmt::Display) -> LogicError {
    LogicError::internal(format!("database query failed: {error}"))
}

impl From<database::Error> for LogicError {
    fn from(error: database::Error) -> Self {
        database_error(error)
    }
}

impl From<CreateArticleError> for LogicError {
    fn from(error: CreateArticleError) -> Self {
        match error {
            CreateArticleError::AuthorMissing => LogicError::internal("author not found"),
            CreateArticleError::TitleTaken => LogicError::bad_request("title already exists"),
            CreateArticleError::ContentHashTaken => {
                LogicError::bad_request("identical PDF already exists")
            }
            CreateArticleError::Db(error) => database_error(error),
        }
    }
}

impl From<UpdateArticleError> for LogicError {
    fn from(error: UpdateArticleError) -> Self {
        match error {
            UpdateArticleError::Missing => LogicError::not_found("article not found"),
            UpdateArticleError::TitleTaken => LogicError::bad_request("title already exists"),
            UpdateArticleError::Db(error) => database_error(error),
        }
    }
}

impl From<CreateCommentError> for LogicError {
    fn from(error: CreateCommentError) -> Self {
        match error {
            CreateCommentError::TargetNotFound => LogicError::not_found(
                "comment target not found (the version may have been removed)",
            ),
            CreateCommentError::CommentIdExists => {
                LogicError::internal("comment id already exists")
            }
            CreateCommentError::CommentTreeTooDeep => LogicError::bad_request(format!(
                "comment thread too deep (max {MAX_COMMENT_TREE_DEPTH} reply layers)"
            )),
            CreateCommentError::Db(error) => database_error(error),
        }
    }
}

impl From<CreateVersionError> for LogicError {
    fn from(error: CreateVersionError) -> Self {
        match error {
            CreateVersionError::ArticleMissing => LogicError::not_found("article not found"),
            CreateVersionError::NotGreater => LogicError::bad_request(
                "new version must be strictly greater than the latest version",
            ),
            CreateVersionError::InvalidNumber => LogicError::bad_request("invalid version number"),
            CreateVersionError::ContentHashTaken => {
                LogicError::bad_request("identical PDF already exists")
            }
            CreateVersionError::Db(error) => database_error(error),
        }
    }
}

impl From<TransferError> for LogicError {
    fn from(error: TransferError) -> Self {
        match error {
            TransferError::NoRecycler => LogicError::internal("no recycler available"),
            TransferError::Db(error) => {
                LogicError::internal(format!("failed to transfer account assets: {error}"))
            }
        }
    }
}

impl From<TransferTargetError> for LogicError {
    fn from(error: TransferTargetError) -> Self {
        match error {
            TransferTargetError::TargetMissing => LogicError::not_found("article not found"),
            TransferTargetError::TargetOwnerMissing => LogicError::internal("article has no owner"),
            TransferTargetError::NoRecycler => LogicError::internal("no recycler available"),
            TransferTargetError::Db(error) => database_error(error),
        }
    }
}

impl From<UserWriteError> for LogicError {
    fn from(error: UserWriteError) -> Self {
        match error {
            UserWriteError::AlreadyTaken => LogicError::bad_request("name already taken"),
            UserWriteError::UserMissing => LogicError::unauthorized("user not found"),
            UserWriteError::EmailMismatch => LogicError::internal("unexpected email mismatch"),
            UserWriteError::Db(error) => {
                LogicError::internal(format!("failed to update name: {error}"))
            }
        }
    }
}
