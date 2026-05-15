mod helpers;

use axum::http::{Method, StatusCode};

use helpers::{add_to_cart, create_test_product, create_test_user, request, test_app};
use rust_ecommerce::auth::Role;

#[sqlx::test(migrations = "./migrations")]
async fn test_create_order_from_cart(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (user_id, token) = create_test_user(&pool, "buyer@example.com", Role::Customer).await;
    let product_id = create_test_product(&pool, "Gadget", 5000, 10).await;
    add_to_cart(&pool, user_id, product_id, 2).await;

    let (status, body) = request(app, Method::POST, "/api/orders", None, Some(&token)).await;

    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_eq!(body["status"], "pending");
    assert_eq!(body["total_amount"], 10000); // 5000 * 2

    // Verify stock was decremented
    let row: (i32,) = sqlx::query_as("SELECT stock_quantity FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, 8, "stock should be 10 - 2 = 8");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_order_empty_cart(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (_, token) = create_test_user(&pool, "empty@example.com", Role::Customer).await;

    let (status, _) = request(app, Method::POST, "/api/orders", None, Some(&token)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_order_insufficient_stock(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (user_id, token) = create_test_user(&pool, "stockout@example.com", Role::Customer).await;
    let product_id = create_test_product(&pool, "Rare Item", 9999, 1).await;
    add_to_cart(&pool, user_id, product_id, 5).await; // want 5, only 1 in stock

    let (status, _) = request(app, Method::POST, "/api/orders", None, Some(&token)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_admin_sees_all_orders(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (user_id, user_token) =
        create_test_user(&pool, "orderer@example.com", Role::Customer).await;
    let (_, admin_token) = create_test_user(&pool, "orders_admin@example.com", Role::Admin).await;
    let product_id = create_test_product(&pool, "Thingamajig", 1000, 20).await;
    add_to_cart(&pool, user_id, product_id, 1).await;

    // Create an order as the customer
    request(
        test_app(pool.clone()),
        Method::POST,
        "/api/orders",
        None,
        Some(&user_token),
    )
    .await;

    // Admin should see it
    let (status, body) = request(app, Method::GET, "/api/orders", None, Some(&admin_token)).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(body.as_array().map(|a| !a.is_empty()).unwrap_or(false));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_customer_cannot_see_all_orders(pool: sqlx::PgPool) {
    let app = test_app(pool.clone());
    let (_, token) = create_test_user(&pool, "nospy@example.com", Role::Customer).await;

    let (status, _) = request(app, Method::GET, "/api/orders", None, Some(&token)).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}
