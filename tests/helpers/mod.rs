use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use rust_ecommerce::auth::{generate_access_token, JwtKeys, Role};
use rust_ecommerce::config::Config;
use rust_ecommerce::{build_app, build_app_state};

/// Test-specific config: mock Paystack, permissive CORS.
pub fn test_config() -> Arc<Config> {
    Arc::new(Config {
        database_url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/rust_ecommerce".to_string()),
        host: "127.0.0.1".to_string(),
        port: "0".to_string(),
        jwt_signing_key: "test-signing-key-for-tests-only-32chars!".to_string(),
        paystack_secret_key: "sk_test_placeholder".to_string(),
        paystack_mock: true,
        allowed_origins: String::new(),
        rust_log: "error".to_string(),
    })
}

pub fn test_app(pool: PgPool) -> Router {
    let config = test_config();
    let state = build_app_state(pool, config);
    build_app(state)
}

/// Issue a request against a cloned Router and return (status, JSON body).
pub async fn request(
    app: Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);

    if let Some(t) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }

    let req_body = match body {
        Some(v) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(serde_json::to_vec(&v).unwrap())
        }
        None => Body::empty(),
    };

    let response = app
        .oneshot(builder.body(req_body).unwrap())
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// Insert a test user and return (user_id, access_token).
pub async fn create_test_user(pool: &PgPool, email: &str, role: Role) -> (Uuid, String) {
    let password_hash = bcrypt::hash("TestPass123!", bcrypt::DEFAULT_COST).unwrap();
    let user_id = Uuid::new_v4();
    let config = test_config();
    let keys = JwtKeys::new(config.jwt_signing_key.as_bytes());

    let role_str = match role {
        Role::Admin => "admin",
        Role::Customer => "customer",
    };

    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role, is_active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5::user_role, TRUE, NOW(), NOW())",
    )
    .bind(user_id)
    .bind(email)
    .bind("Test User")
    .bind(&password_hash)
    .bind(role_str)
    .execute(pool)
    .await
    .expect("failed to insert test user");

    let token = generate_access_token(user_id, email, role, &keys)
        .expect("failed to generate test token");

    (user_id, token)
}

/// Insert a test product and return its UUID.
pub async fn create_test_product(pool: &PgPool, name: &str, price: i32, stock: i32) -> Uuid {
    let product_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO products (id, name, description, price, stock_quantity, is_active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, TRUE, NOW(), NOW())",
    )
    .bind(product_id)
    .bind(name)
    .bind("A test product")
    .bind(price)
    .bind(stock)
    .execute(pool)
    .await
    .expect("failed to insert test product");

    product_id
}

/// Add a product to a user's cart.
pub async fn add_to_cart(pool: &PgPool, user_id: Uuid, product_id: Uuid, quantity: i32) {
    sqlx::query(
        "INSERT INTO cart_items (id, user_id, product_id, quantity, created_at, updated_at)
         VALUES (gen_random_uuid(), $1, $2, $3, NOW(), NOW())
         ON CONFLICT (user_id, product_id) DO UPDATE SET quantity = EXCLUDED.quantity",
    )
    .bind(user_id)
    .bind(product_id)
    .bind(quantity)
    .execute(pool)
    .await
    .expect("failed to add to cart");
}
