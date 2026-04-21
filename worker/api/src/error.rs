use std::fmt::Display;

use axum::{
    extract::{FromRequestParts, Query},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use tracing::error;

use crate::schema::{ApiErrorCode, ApiErrorDetailDto, ApiErrorResponseDto};

pub type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ApiErrorResponseDto,
}

impl ApiError {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::InvalidRequest,
            message,
        )
    }

    #[allow(dead_code)]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, ApiErrorCode::NotFound, message)
    }

    pub fn internal(error: impl Display) -> Self {
        error!(error = %error, "API request failed");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::InternalError,
            "internal server error",
        )
    }

    fn new(status: StatusCode, code: ApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ApiErrorResponseDto {
                error: ApiErrorDetailDto {
                    code,
                    message: message.into(),
                },
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

#[derive(Debug)]
pub struct ApiQuery<T>(pub T);

impl<T> ApiQuery<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<S, T> FromRequestParts<S> for ApiQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|error| ApiError::invalid_request(error.to_string()))?;

        Ok(Self::new(value))
    }
}

pub fn internal_error(error: impl Display) -> ApiError {
    ApiError::internal(error)
}
