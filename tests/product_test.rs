mod helpers;

use axum::http::{Method, StatusCode};
use serde_json::json;

use helpers::{create_test_product, create_test_user, request, test_app};
use rust_ecommerce::auth::Role;

#[sqlx::test(migrations = "./migrations")]
async fn test_get_products_public(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    create_test_product(&pool, "Widget", 1000, 50).await;

    let (status, body) = request(app, Method::GET, "/api/products", None, None).await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(body.as_array().map(|a| !a.is_empty()).unwrap_or(false));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_product_admin(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (_, token) = create_test_user(&pool, "admin@example.com", Role::Admin).await;

    let (status, body) = request(
        app,
        Method::POST,
        "/api/products",
        Some(json!({
            "name": "New Widget",
            "price": 2500,
            "stock_quantity": 100
        })),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_eq!(body["name"], "New Widget");
    assert_eq!(body["price"], 2500);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_product_customer_forbidden(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (_, token) = create_test_user(&pool, "customer@example.com", Role::Customer).await;

    let (status, _) = request(
        app,
        Method::POST,
        "/api/products",
        Some(json!({ "name": "Sneaky Widget", "price": 100, "stock_quantity": 5 })),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_product_unauthenticated(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (status, _) = request(
        app,
        Method::POST,
        "/api/products",
        Some(json!({ "name": "X", "price": 100, "stock_quantity": 1 })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_soft_delete_hides_product(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (_, token) = create_test_user(&pool, "admin2@example.com", Role::Admin).await;
    let product_id = create_test_product(&pool, "Doomed Widget", 500, 10).await;

    // Delete the product
    let (del_status, _) = request(
        app.clone(),
        Method::DELETE,
        &format!("/api/products/{product_id}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(del_status, StatusCode::NO_CONTENT);

    // Deleted product should not appear in public listing
    let (status, body) = request(app, Method::GET, "/api/products", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let products = body.as_array().unwrap();
    assert!(
        products.iter().all(|p| p["id"] != product_id.to_string()),
        "deleted product should not be in public listing"
    );
}
