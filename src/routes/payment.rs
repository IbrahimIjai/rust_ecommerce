use crate::handlers::payment::{initialize_payment, verify_payment};
use crate::handlers::webhook::paystack_webhook;
use crate::services::AppState;
use axum::{routing::post, Router};

pub fn create_payment_routes() -> Router<AppState> {
    Router::new()
        .route("/initialize", post(initialize_payment))
        .route("/verify", post(verify_payment))
        .route("/webhook", post(paystack_webhook))
}
