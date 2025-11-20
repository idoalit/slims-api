use axum::{
    Json,
    http::StatusCode,
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
            AppError::Forbidden(message) => (
                StatusCode::FORBIDDEN,
                "Forbidden",
                Some(message.clone()),
            ),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not Found", Some("not found".into())),
            AppError::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                "Bad Request",
                Some(message.clone()),
            ),
            AppError::Database(err) => {
                if let sqlx::Error::RowNotFound = err {
                    (StatusCode::NOT_FOUND, "Not Found", Some("not found".into()))
                } else {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Database Error",
                        None,
                    )
                }
            }
            AppError::Jwt(_) => (
                StatusCode::UNAUTHORIZED,
                "Invalid Token",
                Some("invalid token".into()),
            ),
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Error",
                None,
            ),
        };

        let error = JsonApiError {
            status: status.as_u16().to_string(),
            title: Some(title.into()),
            detail,
        };

        let body = Json(JsonApiErrorDocument { errors: vec![error] });
        (status, body).into_response()
    }
}
