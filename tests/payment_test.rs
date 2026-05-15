mod helpers;

use axum::http::{Method, StatusCode};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha512;

use helpers::{add_to_cart, create_test_product, create_test_user, request, test_app};
use rust_ecommerce::auth::Role;

#[sqlx::test(migrations = "./migrations")]
async fn test_initialize_payment_mock(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (user_id, token) = create_test_user(&pool, "payer@example.com", Role::Customer).await;
    let product_id = create_test_product(&pool, "Payable Item", 3000, 5).await;
    add_to_cart(&pool, user_id, product_id, 1).await;

    // Create an order first
    let (_, order_body) = request(
        test_app(pool.clone()),
        Method::POST,
        "/api/orders",
        None,
        Some(&token),
    )
    .await;
    let order_id = order_body["id"].as_str().unwrap();

    let (status, body) = request(
        app,
        Method::POST,
        "/api/payment/initialize",
        Some(json!({
            "order_id": order_id,
            "email": "payer@example.com"
        })),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(body["data"]["reference"].is_string());
    assert!(body["data"]["authorization_url"]
        .as_str()
        .unwrap_or("")
        .contains("mock-checkout"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_verify_payment_mock(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (user_id, token) = create_test_user(&pool, "verifier@example.com", Role::Customer).await;
    let product_id = create_test_product(&pool, "Verified Item", 2000, 5).await;
    add_to_cart(&pool, user_id, product_id, 1).await;

    // Create order
    let (_, order_body) = request(
        test_app(pool.clone()),
        Method::POST,
        "/api/orders",
        None,
        Some(&token),
    )
    .await;
    let order_id = order_body["id"].as_str().unwrap();

    // Initialize payment to get reference
    let (_, init_body) = request(
        test_app(pool.clone()),
        Method::POST,
        "/api/payment/initialize",
        Some(json!({
            "order_id": order_id,
            "email": "verifier@example.com"
        })),
        Some(&token),
    )
    .await;
    let reference = init_body["data"]["reference"].as_str().unwrap().to_string();

    // Verify payment (mock mode — sets status to paid)
    let (status, body) = request(
        app,
        Method::POST,
        "/api/payment/verify",
        Some(json!({ "reference": reference })),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["order_id"].as_str().unwrap(), order_id);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_webhook_invalid_signature(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let payload = json!({
        "event": "charge.success",
        "data": { "reference": "ref_test_123" }
    });

    let (status, _) = request(
        app,
        Method::POST,
        "/api/payment/webhook",
        Some(payload),
        None, // no Authorization header needed, but we skip the signature header
    )
    .await;

    // Missing x-paystack-signature → 401
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_webhook_valid_signature(pool: sqlx::PgPool) {
    use axum::body::Body;
    use axum::http::{header, Request};
    use tower::ServiceExt;

    let pool_clone = pool.clone();
    let app = test_app(pool_clone);

    let payload = json!({
        "event": "charge.success",
        "data": { "reference": "ref_valid_sig_test" }
    });
    let body_bytes = serde_json::to_vec(&payload).unwrap();

    // Compute HMAC-SHA512 using the test secret key
    let secret = "sk_test_placeholder";
    let mut mac = Hmac::<Sha512>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(&body_bytes);
    let signature = hex::encode(mac.finalize().into_bytes());

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/payment/webhook")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-paystack-signature", &signature)
        .body(Body::from(body_bytes))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_webhook_idempotency(pool: sqlx::PgPool) {
    use axum::body::Body;
    use axum::http::{header, Request};
    use tower::ServiceExt;

    let payload = json!({
        "event": "charge.success",
        "data": { "reference": "ref_idempotency_test" }
    });
    let body_bytes = serde_json::to_vec(&payload).unwrap();

    let secret = "sk_test_placeholder";
    let mut mac = Hmac::<Sha512>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(&body_bytes);
    let signature = hex::encode(mac.finalize().into_bytes());

    // First call
    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/payment/webhook")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-paystack-signature", &signature)
        .body(Body::from(body_bytes.clone()))
        .unwrap();
    let r1 = test_app(pool.clone()).oneshot(req1).await.unwrap();
    assert_eq!(r1.status(), StatusCode::OK);

    // Second call (duplicate) — must also return 200 (idempotent)
    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/payment/webhook")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-paystack-signature", &signature)
        .body(Body::from(body_bytes))
        .unwrap();
    let r2 = test_app(pool.clone()).oneshot(req2).await.unwrap();
    assert_eq!(r2.status(), StatusCode::OK);

    // Verify only one row was inserted (idempotency check)
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM webhook_events WHERE reference = $1")
            .bind("ref_idempotency_test")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1, "duplicate webhook should produce exactly one DB row");
}
