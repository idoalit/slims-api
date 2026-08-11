use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::jsonapi::{JsonApiError, JsonApiErrorDocument};

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum AppError {
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("too many requests: {0}")]
    TooManyRequests(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("internal error: {0}")]
    Internal(String),
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, title, detail) = match &self {
            AppError::Unauthorized(message) => (
                StatusCode::UNAUTHORIZED,
                "Unauthorized",
                Some(message.clone()),
            ),
            AppError::Forbidden(message) => {
                (StatusCode::FORBIDDEN, "Forbidden", Some(message.clone()))
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not Found", Some("not found".into())),
            AppError::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                "Bad Request",
                Some(message.clone()),
            ),
            AppError::TooManyRequests(message) => (
                StatusCode::TOO_MANY_REQUESTS,
                "Too Many Requests",
                Some(message.clone()),
            ),
            AppError::Conflict(message) => {
                (StatusCode::CONFLICT, "Conflict", Some(message.clone()))
            }
            AppError::Database(err) => {
                if let sqlx::Error::RowNotFound = err {
                    (StatusCode::NOT_FOUND, "Not Found", Some("not found".into()))
                } else if matches!(
                    err,
                    sqlx::Error::Database(database_error)
                        if database_error.is_unique_violation()
                ) {
                    (
                        StatusCode::CONFLICT,
                        "Conflict",
                        Some("a resource with the same unique value already exists".into()),
                    )
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Database Error", None)
                }
            }
            AppError::Jwt(_) => (
                StatusCode::UNAUTHORIZED,
                "Invalid Token",
                Some("invalid token".into()),
            ),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error", None),
        };

        let error = JsonApiError {
            status: status.as_u16().to_string(),
            title: Some(title.into()),
            detail,
        };

        let body = Json(JsonApiErrorDocument {
            errors: vec![error],
        });
        let mut response = (status, body).into_response();
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, max-age=0"),
        );
        if matches!(self, AppError::TooManyRequests(_)) {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("900"));
        }
        response
    }
}
