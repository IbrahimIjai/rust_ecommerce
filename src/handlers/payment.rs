use axum::{extract::State, response::Json};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::Order;
use crate::services::{DbPool, PaystackService};

#[derive(serde::Deserialize)]
pub struct InitializePaymentRequest {
    pub order_id: Uuid,
    pub email: String,
}

#[derive(serde::Deserialize)]
pub struct VerifyPaymentRequest {
    pub reference: String,
}

pub async fn initialize_payment(
    State(pool): State<DbPool>,
    State(paystack): State<PaystackService>,
    Json(body): Json<InitializePaymentRequest>,
) -> Result<Json<Value>, AppError> {
    let order = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1")
        .bind(body.order_id)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    if order.status != "pending" {
        return Err(AppError::BadRequest(
            "Order is not pending payment".to_string(),
        ));
    }

    let reference = PaystackService::generate_reference();

    if paystack.is_mock_mode() {
        let mut tx = pool.begin().await.map_err(AppError::from)?;

        sqlx::query("UPDATE orders SET payment_reference = $1 WHERE id = $2")
            .bind(&reference)
            .bind(order.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update payment reference (mock): {}", e);
                AppError::from(e)
            })?;

        tx.commit().await.map_err(AppError::from)?;

        return Ok(Json(json!({
            "status": true,
            "message": "Payment initialized successfully (mock mode)",
            "data": {
                "authorization_url": format!("https://example.com/mock-checkout?reference={}", reference),
                "access_code": "mock_access_code",
                "reference": reference
            }
        })));
    }

    let response = paystack
        .initialize_payment(&body.email, order.total_amount, &reference)
        .await
        .map_err(|e| {
            tracing::error!("Paystack initialize_payment failed: {}", e);
            AppError::InternalServerError("Failed to initialize payment".to_string())
        })?;

    if !response.status {
        return Err(AppError::BadRequest(response.message));
    }

    let mut tx = pool.begin().await.map_err(AppError::from)?;

    sqlx::query("UPDATE orders SET payment_reference = $1 WHERE id = $2")
        .bind(&reference)
        .bind(order.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update payment reference: {}", e);
            AppError::from(e)
        })?;

    tx.commit().await.map_err(AppError::from)?;

    Ok(Json(json!({
        "status": true,
        "message": "Payment initialized successfully",
        "data": {
            "authorization_url": response.data.authorization_url,
            "access_code": response.data.access_code,
            "reference": response.data.reference
        }
    })))
}

pub async fn verify_payment(
    State(pool): State<DbPool>,
    State(paystack): State<PaystackService>,
    Json(body): Json<VerifyPaymentRequest>,
) -> Result<Json<Value>, AppError> {
    let reference = &body.reference;

    if paystack.is_mock_mode() {
        let mut tx = pool.begin().await.map_err(AppError::from)?;

        let order = sqlx::query_as::<_, Order>(
            "SELECT * FROM orders WHERE payment_reference = $1",
        )
        .bind(reference)
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("Order not found for this payment reference".to_string()))?;

        sqlx::query("UPDATE orders SET status = $1 WHERE id = $2")
            .bind("paid")
            .bind(order.id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::from)?;

        tx.commit().await.map_err(AppError::from)?;

        return Ok(Json(json!({
            "status": true,
            "message": "Payment verified successfully (mock mode)",
            "data": {
                "order_id": order.id,
                "amount": order.total_amount,
                "paid_at": chrono::Utc::now().to_rfc3339()
            }
        })));
    }

    let response = paystack.verify_payment(reference).await.map_err(|e| {
        tracing::error!("Paystack verify_payment failed: {}", e);
        AppError::InternalServerError("Failed to verify payment".to_string())
    })?;

    if !response.status || response.data.status != "success" {
        return Err(AppError::BadRequest("Payment was not successful".to_string()));
    }

    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let order = sqlx::query_as::<_, Order>(
        "SELECT * FROM orders WHERE payment_reference = $1",
    )
    .bind(reference)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::NotFound("Order not found for this payment reference".to_string()))?;

    sqlx::query("UPDATE orders SET status = $1 WHERE id = $2")
        .bind("paid")
        .bind(order.id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;

    Ok(Json(json!({
        "status": true,
        "message": "Payment verified successfully",
        "data": {
            "order_id": order.id,
            "amount": response.data.amount,
            "paid_at": response.data.paid_at
        }
    })))
}
