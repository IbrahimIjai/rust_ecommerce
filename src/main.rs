use std::sync::Arc;

use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use rust_ecommerce::{build_app, build_app_state, config::Config, services::create_connection_pool, services::run_migrations};

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

    let server_address = config.server_address();
    let state = build_app_state(db_pool, Arc::new(config));
    let app = build_app(state);

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
