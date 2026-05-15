use axum::{routing::post, Router};

use crate::handlers::auth::{forgot_password, login, me, refresh_token, reset_password, signup};
use crate::services::AppState;

pub fn create_auth_routes() -> Router<AppState> {
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
        .route("/me", axum::routing::get(me))
}
