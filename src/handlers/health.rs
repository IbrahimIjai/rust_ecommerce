use axum::{extract::State, response::Json};
use serde_json::{json, Value};
use std::time::Instant;

use crate::error::AppError;
use crate::services::DbPool;

#[utoipa::path(
    get, path = "/api/health", tag = "Health",
    responses(
        (status = 200, description = "Service and database are healthy", body = serde_json::Value),
        (status = 503, description = "Database health check failed"),
    )
)]
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
