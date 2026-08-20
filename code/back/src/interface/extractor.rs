use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::infrastructure::state::AppState;
use crate::interface::envelope::ApiError;

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

#[derive(Deserialize)]
struct PagedQueryParams {
    page: Option<u64>,
    limit: Option<u64>,
}

pub struct AppPaged(pub (u64, u64));

impl FromRequestParts<AppState> for AppPaged {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let params = axum::extract::Query::<PagedQueryParams>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::bad_request("invalid query parameters"))?;
        let (page, limit) = crate::logic::pagination::clamp_page_limit(
            params.page,
            params.limit,
            state.configurator.search_page_size(),
            state.configurator.max_search_pages(),
        )
        .map_err(ApiError::from_logic)?;
        Ok(Self((page, limit)))
    }
}
