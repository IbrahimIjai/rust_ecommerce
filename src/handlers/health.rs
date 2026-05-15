use axum::{extract::State, response::Json};
use serde_json::{json, Value};

use crate::error::AppError;
use crate::services::{check_database_health, DbPool};

pub async fn health_check(State(pool): State<DbPool>) -> Result<Json<Value>, AppError> {
    check_database_health(&pool)
        .await
        .map_err(|e| AppError::ServiceUnavailable(e.to_string()))?;

    Ok(Json(json!({
        "status": "healthy",
        "database": "connected",
        "version": env!("CARGO_PKG_VERSION")
    })))
}
