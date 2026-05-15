use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::product::{
    create_product, delete_product, get_product, get_products, update_product,
};
use crate::services::AppState;

pub fn create_product_routes() -> Router<AppState> {
    Router::new()
        // Public
        .route("/", get(get_products))
        .route("/:id", get(get_product))
        // Admin only
        .route("/", post(create_product))
        .route("/:id", put(update_product))
        .route("/:id", delete(delete_product))
}
