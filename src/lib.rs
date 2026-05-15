pub mod auth;
pub mod config;
pub mod error;
pub mod extractors;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    trace::TraceLayer,
};

use auth::JwtKeys;
use config::Config;
use routes::create_routes;
use services::{AppState, DbPool, PaystackService};

pub fn build_app(state: AppState) -> Router {
    let cors = build_cors(&state.config.allowed_origins.clone());

    Router::new()
        .route(
            "/",
            get(|| async {
                axum::Json(serde_json::json!({
                    "message": "Rust E-commerce API",
                    "version": env!("CARGO_PKG_VERSION"),
                    "docs": "/api/health"
                }))
            }),
        )
        .nest("/api", create_routes())
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .into_inner(),
        )
}

pub fn build_app_state(db_pool: DbPool, config: Arc<Config>) -> AppState {
    let jwt_keys = Arc::new(JwtKeys::new(config.jwt_signing_key.as_bytes()));
    let paystack_service = PaystackService::new();
    AppState::new(db_pool, paystack_service, jwt_keys, config)
}

fn build_cors(allowed_origins: &str) -> CorsLayer {
    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|o| o.trim().parse::<HeaderValue>().ok())
        .collect();

    if origins.is_empty() {
        return CorsLayer::permissive();
    }

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT])
        .max_age(Duration::from_secs(3600))
}
