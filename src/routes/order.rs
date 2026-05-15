use axum::{routing::get, Router};

use crate::handlers::order::{create_order, get_order, get_orders, get_user_orders};
use crate::services::AppState;

pub fn create_order_routes() -> Router<AppState> {
    Router::new()
        // GET /api/orders        — admin: all orders
        // POST /api/orders       — auth: create from cart (user_id from JWT)
        .route("/", get(get_orders).post(create_order))
        // GET /api/orders/user/:user_id — owner or admin
        .route("/user/:user_id", get(get_user_orders))
        // GET /api/orders/:id   — owner or admin
        .route("/:id", get(get_order))
}
