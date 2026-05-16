use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use constant_time_eq::constant_time_eq;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use std::sync::Arc;

use crate::config::Config;
use crate::error::AppError;
use crate::models::OrderStatus;
use crate::services::DbPool;

type HmacSha512 = Hmac<Sha512>;

fn verify_paystack_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    let Ok(mut mac) = HmacSha512::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    let computed = hex::encode(mac.finalize().into_bytes());
    constant_time_eq(computed.as_bytes(), signature.as_bytes())
}

/// POST /api/payment/webhook — called by Paystack, not user-facing (excluded from OpenAPI docs)
pub async fn paystack_webhook(
    State(pool): State<DbPool>,
    State(config): State<Arc<Config>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    // 1. Verify HMAC-SHA512 signature
    let signature = headers
        .get("x-paystack-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    if !verify_paystack_signature(&config.paystack_secret_key, &body, signature) {
        return Err(AppError::Unauthorized);
    }

    // 2. Parse event payload
    let payload: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;

    let event_type = payload["event"].as_str().unwrap_or("unknown").to_string();
    let reference = payload["data"]["reference"].as_str().map(|s| s.to_string());

    // 3. Idempotency — insert into webhook_events, skip if duplicate
    let insert_result = sqlx::query(
        r#"
        INSERT INTO webhook_events (event_type, reference, payload)
        VALUES ($1, $2, $3)
        ON CONFLICT (reference, event_type) DO NOTHING
        "#,
    )
    .bind(&event_type)
    .bind(&reference)
    .bind(&payload)
    .execute(&pool)
    .await
    .map_err(AppError::from)?;

    // Already processed — return 200 immediately
    if insert_result.rows_affected() == 0 {
        tracing::debug!(event = %event_type, "Duplicate webhook event ignored");
        return Ok(StatusCode::OK);
    }

    // 4. Handle event types
    match event_type.as_str() {
        "charge.success" => {
            if let Some(ref reference) = reference {
                let updated = sqlx::query(
                    "UPDATE orders SET status = $1, updated_at = NOW()
                     WHERE payment_reference = $2 AND status = $3",
                )
                .bind(OrderStatus::Paid)
                .bind(reference)
                .bind(OrderStatus::PaymentInitiated)
                .execute(&pool)
                .await
                .map_err(AppError::from)?;

                if updated.rows_affected() > 0 {
                    tracing::info!(reference = %reference, "Order marked as paid via webhook");
                } else {
                    tracing::warn!(reference = %reference, "charge.success received but no matching order found");
                }
            }
        }
        other => {
            tracing::debug!(event = %other, "Unhandled Paystack webhook event");
        }
    }

    Ok(StatusCode::OK)
}
