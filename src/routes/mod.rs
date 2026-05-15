use crate::handlers::health::health_check;
use crate::services::AppState;
use axum::{routing::get, Router};

pub mod auth;
pub mod cart;
pub mod order;
pub mod payment;
pub mod product;
pub mod user;

pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .nest("/auth", auth::create_auth_routes())
        .nest("/users", user::create_user_routes())
        .nest("/products", product::create_product_routes())
        .nest("/cart", cart::create_cart_routes())
        .nest("/orders", order::create_order_routes())
        .nest("/payment", payment::create_payment_routes())
}
