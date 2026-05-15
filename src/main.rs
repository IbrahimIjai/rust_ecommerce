use std::sync::Arc;

use axum::{routing::get, Router};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod handlers;
mod models;
mod routes;
mod services;

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

    let paystack_service = PaystackService::new();
    let server_address = config.server_address();
    let config = Arc::new(config);

    let app = Router::new()
        .route(
            "/",
            get(|| async {
                axum::Json(serde_json::json!({"message": "Rust E-commerce API", "version": env!("CARGO_PKG_VERSION")}))
            }),
        )
        .nest("/api", create_routes())
        .with_state(AppState::new(db_pool, paystack_service, config))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .into_inner(),
        );

    info!("Starting server at http://{}", server_address);

    let listener = tokio::net::TcpListener::bind(&server_address)
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
