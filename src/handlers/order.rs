use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{CreateOrder, Order, OrderItemResponse, OrderResponse};
use crate::services::DbPool;

#[derive(sqlx::FromRow)]
struct OrderItemRow {
    product_id: Uuid,
    product_name: String,
    product_price: i32,
    quantity: i32,
}

#[derive(sqlx::FromRow)]
struct CartItemForOrder {
    product_id: Uuid,
    quantity: i32,
    product_name: String,
    product_price: i32,
}

async fn fetch_order_items(pool: &DbPool, order_id: Uuid) -> Result<Vec<OrderItemResponse>, AppError> {
    let rows = sqlx::query_as::<_, OrderItemRow>(
        "SELECT oi.product_id, oi.product_name, oi.product_price, oi.quantity
         FROM order_items oi WHERE oi.order_id = $1",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| OrderItemResponse::new(r.product_id, r.product_name, r.product_price, r.quantity))
        .collect())
}

pub async fn get_orders(
    State(pool): State<DbPool>,
) -> Result<Json<Vec<OrderResponse>>, AppError> {
    let orders = sqlx::query_as::<_, Order>("SELECT * FROM orders ORDER BY created_at DESC")
        .fetch_all(&pool)
        .await
        .map_err(AppError::from)?;

    let mut responses = Vec::with_capacity(orders.len());
    for order in orders {
        let items = fetch_order_items(&pool, order.id).await?;
        responses.push(OrderResponse::new(order, items));
    }

    Ok(Json(responses))
}

pub async fn get_user_orders(
    Path(user_id): Path<Uuid>,
    State(pool): State<DbPool>,
) -> Result<Json<Vec<OrderResponse>>, AppError> {
    let orders = sqlx::query_as::<_, Order>(
        "SELECT * FROM orders WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .map_err(AppError::from)?;

    let mut responses = Vec::with_capacity(orders.len());
    for order in orders {
        let items = fetch_order_items(&pool, order.id).await?;
        responses.push(OrderResponse::new(order, items));
    }

    Ok(Json(responses))
}

pub async fn get_order(
    Path((user_id, order_id)): Path<(Uuid, Uuid)>,
    State(pool): State<DbPool>,
) -> Result<Json<OrderResponse>, AppError> {
    let order = sqlx::query_as::<_, Order>(
        "SELECT * FROM orders WHERE id = $1 AND user_id = $2",
    )
    .bind(order_id)
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    let items = fetch_order_items(&pool, order.id).await?;
    Ok(Json(OrderResponse::new(order, items)))
}

pub async fn create_order(
    State(pool): State<DbPool>,
    Json(body): Json<CreateOrder>,
) -> Result<(StatusCode, Json<OrderResponse>), AppError> {
    let user_id = body.user_id;
    let now = chrono::Utc::now();

    let cart_items = sqlx::query_as::<_, CartItemForOrder>(
        r#"
        SELECT ci.product_id, ci.quantity, p.name as product_name, p.price as product_price
        FROM cart_items ci
        JOIN products p ON ci.product_id = p.id
        WHERE ci.user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .map_err(AppError::from)?;

    if cart_items.is_empty() {
        return Err(AppError::BadRequest("Cart is empty".to_string()));
    }

    let total_amount: i32 = cart_items
        .iter()
        .map(|i| i.product_price * i.quantity)
        .sum();

    let mut tx = pool
        .begin()
        .await
        .map_err(AppError::from)?;

    let order_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO orders (id, user_id, total_amount, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(order_id)
    .bind(user_id)
    .bind(total_amount)
    .bind("pending")
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert order: {}", e);
        AppError::from(e)
    })?;

    for item in &cart_items {
        let subtotal = item.product_price * item.quantity;
        sqlx::query(
            "INSERT INTO order_items (id, order_id, product_id, product_name, product_price, quantity, subtotal)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(order_id)
        .bind(item.product_id)
        .bind(&item.product_name)
        .bind(item.product_price)
        .bind(item.quantity)
        .bind(subtotal)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert order item: {}", e);
            AppError::from(e)
        })?;
    }

    sqlx::query("DELETE FROM cart_items WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;

    let order = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .map_err(AppError::from)?;

    let items = fetch_order_items(&pool, order.id).await?;
    Ok((StatusCode::CREATED, Json(OrderResponse::new(order, items))))
}
