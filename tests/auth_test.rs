mod helpers;

use axum::http::{Method, StatusCode};
use serde_json::json;

use helpers::{create_test_user, request, test_app};
use rust_ecommerce::auth::Role;

// ─── Signup ──────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn test_signup_success(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (status, body) = request(
        app,
        Method::POST,
        "/api/auth/signup",
        Some(json!({
            "name": "Alice",
            "email": "alice@example.com",
            "password": "SecurePass123!"
        })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_signup_duplicate_email(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    create_test_user(&pool, "dup@example.com", Role::Customer).await;

    let (status, _) = request(
        app,
        Method::POST,
        "/api/auth/signup",
        Some(json!({
            "name": "Dup",
            "email": "dup@example.com",
            "password": "SecurePass123!"
        })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_signup_invalid_email(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (status, _) = request(
        app,
        Method::POST,
        "/api/auth/signup",
        Some(json!({
            "name": "Bob",
            "email": "not-an-email",
            "password": "SecurePass123!"
        })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_signup_short_password(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (status, _) = request(
        app,
        Method::POST,
        "/api/auth/signup",
        Some(json!({
            "name": "Bob",
            "email": "bob@example.com",
            "password": "short"
        })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ─── Login ───────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn test_login_success(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    create_test_user(&pool, "login@example.com", Role::Customer).await;

    let (status, body) = request(
        app,
        Method::POST,
        "/api/auth/login",
        Some(json!({
            "email": "login@example.com",
            "password": "TestPass123!"
        })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(body["access_token"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_login_wrong_password(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    create_test_user(&pool, "wrong@example.com", Role::Customer).await;

    let (status, _) = request(
        app,
        Method::POST,
        "/api/auth/login",
        Some(json!({
            "email": "wrong@example.com",
            "password": "WrongPassword!"
        })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_login_nonexistent_email(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (status, _) = request(
        app,
        Method::POST,
        "/api/auth/login",
        Some(json!({
            "email": "ghost@example.com",
            "password": "AnyPassword123!"
        })),
        None,
    )
    .await;

    // Same 401 as wrong password — no email enumeration
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ─── Protected routes ────────────────────────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn test_me_authenticated(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (_, token) = create_test_user(&pool, "me@example.com", Role::Customer).await;

    let (status, body) = request(app, Method::GET, "/api/auth/me", None, Some(&token)).await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["email"], "me@example.com");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_me_unauthenticated(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (status, _) = request(app, Method::GET, "/api/auth/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
