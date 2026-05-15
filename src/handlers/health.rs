use axum::{extract::State, response::Json};
use serde_json::{json, Value};
use std::time::Instant;

use crate::error::AppError;
use crate::services::DbPool;

pub async fn health_check(State(pool): State<DbPool>) -> Result<Json<Value>, AppError> {
    let start = Instant::now();
    sqlx::query("SELECT 1")
        .fetch_one(&pool)
        .await
        .map_err(|e| AppError::ServiceUnavailable(e.to_string()))?;
    let latency_ms = start.elapsed().as_millis();

    Ok(Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "database": {
            "status": "connected",
            "latency_ms": latency_ms
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
