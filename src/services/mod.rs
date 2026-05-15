pub mod database;
pub mod payment;

pub use database::{check_database_health, create_connection_pool, run_migrations, DbPool};
pub use payment::PaystackService;

use axum::extract::FromRef;
use std::sync::Arc;

use crate::auth::JwtKeys;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
    pub paystack_service: PaystackService,
    pub jwt_keys: Arc<JwtKeys>,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(
        db_pool: DbPool,
        paystack_service: PaystackService,
        jwt_keys: Arc<JwtKeys>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            db_pool,
            paystack_service,
            jwt_keys,
            config,
        }
    }
}

impl FromRef<AppState> for DbPool {
    fn from_ref(state: &AppState) -> Self {
        state.db_pool.clone()
    }
}

impl FromRef<AppState> for PaystackService {
    fn from_ref(state: &AppState) -> Self {
        state.paystack_service.clone()
    }
}

impl FromRef<AppState> for Arc<JwtKeys> {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_keys.clone()
    }
}

impl FromRef<AppState> for Arc<Config> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}
