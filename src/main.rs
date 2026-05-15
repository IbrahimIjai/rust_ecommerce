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
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod config;
mod error;
mod extractors;
mod handlers;
mod models;
mod routes;
mod services;

use auth::JwtKeys;
use config::Config;
use routes::create_routes;
use services::{create_connection_pool, run_migrations, AppState, PaystackService};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&config.rust_log))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_pool = match create_connection_pool().await {
        Ok(pool) => {
            info!("Database connection pool created successfully");
            pool
        }
        Err(e) => {
            tracing::error!("Failed to create database connection pool: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = run_migrations(&db_pool).await {
        tracing::error!("Failed to run migrations: {}", e);
        std::process::exit(1);
    }
    info!("Database migrations applied successfully");

    let jwt_keys = Arc::new(JwtKeys::new(config.jwt_signing_key.as_bytes()));
    let paystack_service = PaystackService::new();
    let server_address = config.server_address();

    let cors = build_cors(&config.allowed_origins);
    let config = Arc::new(config);

    let app = Router::new()
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
        .with_state(AppState::new(db_pool, paystack_service, jwt_keys, config))
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .into_inner(),
        );

    info!("Starting server at http://{}", server_address);

    let listener = tokio::net::TcpListener::bind(&server_address)
        .await
        .unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    info!("Server shut down gracefully");
}

fn build_cors(allowed_origins: &str) -> CorsLayer {
    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|o| o.trim().parse::<HeaderValue>().ok())
        .collect();

    if origins.is_empty() {
        tracing::warn!("No valid ALLOWED_ORIGINS configured — falling back to permissive CORS");
        return CorsLayer::permissive();
    }

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT])
        .max_age(Duration::from_secs(3600))
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { info!("Received Ctrl+C, shutting down..."); },
        _ = terminate => { info!("Received SIGTERM, shutting down..."); },
    }
}
