use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;

use crate::interface::envelope::ApiError;

/// JSON body extractor that maps every axum rejection to the 400 envelope.
pub struct AppJson<T>(pub T);

impl<T, S> FromRequest<S> for AppJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        axum::Json::<T>::from_request(req, state)
            .await
            .map(|json| AppJson(json.0))
            .map_err(|_| ApiError::bad_request("invalid request body"))
    }
}

/// Query-string extractor that maps every axum rejection to the 400 envelope.
pub struct AppQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for AppQuery<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|query| AppQuery(query.0))
            .map_err(|_| ApiError::bad_request("invalid query parameters"))
    }
}

/// Path-parameter extractor that maps every axum rejection to the 400 envelope.
pub struct AppPath<T>(pub T);

impl<T, S> FromRequestParts<S> for AppPath<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map(|path| AppPath(path.0))
            .map_err(|_| ApiError::bad_request("invalid path parameters"))
    }
}

/// Multipart extractor that maps every axum rejection to the 400 envelope.
pub struct AppMultipart(pub axum::extract::Multipart);

impl<S> FromRequest<S> for AppMultipart
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Multipart::from_request(req, state)
            .await
            .map(AppMultipart)
            .map_err(|_| ApiError::bad_request("invalid multipart body"))
    }
}
