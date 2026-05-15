use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    InternalServerError(String),

    #[error("Service unavailable")]
    ServiceUnavailable(String),

    #[error("Validation failed")]
    UnprocessableEntity(Vec<FieldError>),
}

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, json!({"error": msg})),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, json!({"error": msg})),
            AppError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, json!({"error": "Unauthorized"}))
            }
            AppError::Forbidden => (StatusCode::FORBIDDEN, json!({"error": "Forbidden"})),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, json!({"error": msg})),
            AppError::InternalServerError(msg) => {
                tracing::error!("Internal server error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({"error": "An internal error occurred"}),
                )
            }
            AppError::ServiceUnavailable(msg) => {
                tracing::error!("Service unavailable: {}", msg);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    json!({"error": "Service temporarily unavailable"}),
                )
            }
            AppError::UnprocessableEntity(errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error": "Validation failed", "fields": errors}),
            ),
        };

        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource not found".to_string()),
            e => AppError::InternalServerError(e.to_string()),
        }
    }
}
