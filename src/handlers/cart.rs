use axum::{
    extract::{Path, State},
    response::Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::auth::{Claims, Role};
use crate::error::AppError;
use crate::models::{AddToCart, CartItemResponse, CartResponse, UpdateCartItem};
use crate::services::DbPool;

#[derive(sqlx::FromRow)]
struct CartItemRow {
    id: Uuid,
    product_id: Uuid,
    quantity: i32,
    product_name: String,
    product_price: i32,
}

#[derive(sqlx::FromRow)]
struct CartItemBasic {
    id: Uuid,
    quantity: i32,
}

fn check_cart_ownership(claims: &Claims, path_user_id: Uuid) -> Result<(), AppError> {
    if claims.role != Role::Admin && claims.user_id()? != path_user_id {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

#[utoipa::path(
    get, path = "/api/cart/{user_id}", tag = "Cart",
    params(("user_id" = Uuid, Path, description = "User ID whose cart should be returned")),
    responses(
        (status = 200, description = "Cart contents", body = CartResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_cart(
    Path(user_id): Path<Uuid>,
    claims: Claims,
    State(pool): State<DbPool>,
) -> Result<Json<CartResponse>, AppError> {
    check_cart_ownership(&claims, user_id)?;

    let rows = sqlx::query_as::<_, CartItemRow>(
        r#"
        SELECT ci.id, ci.product_id, ci.quantity,
               p.name as product_name, p.price as product_price
        FROM cart_items ci
        JOIN products p ON ci.product_id = p.id
        WHERE ci.user_id = $1
        ORDER BY ci.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .map_err(AppError::from)?;

    let items = rows
        .into_iter()
        .map(|r| {
            CartItemResponse::new(
                r.id,
                r.product_id,
                r.product_name,
                r.product_price,
                r.quantity,
            )
        })
        .collect();

    Ok(Json(CartResponse::new(items)))
}

#[utoipa::path(
    post, path = "/api/cart/{user_id}", tag = "Cart",
    params(("user_id" = Uuid, Path, description = "User ID whose cart should receive the item")),
    request_body = AddToCart,
    responses(
        (status = 200, description = "Item added to cart or existing quantity updated", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin only"),
        (status = 404, description = "Product not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn add_to_cart(
    Path(user_id): Path<Uuid>,
    claims: Claims,
    State(pool): State<DbPool>,
    Json(body): Json<AddToCart>,
) -> Result<Json<serde_json::Value>, AppError> {
    check_cart_ownership(&claims, user_id)?;

    let now = chrono::Utc::now();

    let product_exists = sqlx::query("SELECT id FROM products WHERE id = $1")
        .bind(body.product_id)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?;

    if product_exists.is_none() {
        return Err(AppError::NotFound("Product not found".to_string()));
    }

    let existing = sqlx::query_as::<_, CartItemBasic>(
        "SELECT id, quantity FROM cart_items WHERE user_id = $1 AND product_id = $2",
    )
    .bind(user_id)
    .bind(body.product_id)
    .fetch_optional(&pool)
    .await
    .map_err(AppError::from)?;

    if let Some(item) = existing {
        let new_qty = item.quantity + body.quantity;
        sqlx::query("UPDATE cart_items SET quantity = $1, updated_at = $2 WHERE id = $3")
            .bind(new_qty)
            .bind(now)
            .bind(item.id)
            .execute(&pool)
            .await
            .map_err(AppError::from)?;

        Ok(Json(
            json!({"message": "Cart item updated successfully", "quantity": new_qty}),
        ))
    } else {
        let new_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO cart_items (id, user_id, product_id, quantity, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(new_id)
        .bind(user_id)
        .bind(body.product_id)
        .bind(body.quantity)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .map_err(AppError::from)?;

        Ok(Json(
            json!({"message": "Item added to cart successfully", "cart_item_id": new_id}),
        ))
    }
}

#[utoipa::path(
    put, path = "/api/cart/{user_id}/{item_id}", tag = "Cart",
    params(
        ("user_id" = Uuid, Path, description = "User ID that owns the cart item"),
        ("item_id" = Uuid, Path, description = "Cart item ID"),
    ),
    request_body = UpdateCartItem,
    responses(
        (status = 200, description = "Cart item updated or removed when quantity is zero", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin only"),
        (status = 404, description = "Cart item not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_cart_item(
    Path((user_id, item_id)): Path<(Uuid, Uuid)>,
    claims: Claims,
    State(pool): State<DbPool>,
    Json(body): Json<UpdateCartItem>,
) -> Result<Json<serde_json::Value>, AppError> {
    check_cart_ownership(&claims, user_id)?;

    let now = chrono::Utc::now();

    let exists = sqlx::query("SELECT id FROM cart_items WHERE id = $1 AND user_id = $2")
        .bind(item_id)
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?;

    if exists.is_none() {
        return Err(AppError::NotFound("Cart item not found".to_string()));
    }

    if body.quantity <= 0 {
        sqlx::query("DELETE FROM cart_items WHERE id = $1")
            .bind(item_id)
            .execute(&pool)
            .await
            .map_err(AppError::from)?;
        return Ok(Json(json!({"message": "Cart item removed successfully"})));
    }

    sqlx::query("UPDATE cart_items SET quantity = $1, updated_at = $2 WHERE id = $3")
        .bind(body.quantity)
        .bind(now)
        .bind(item_id)
        .execute(&pool)
        .await
        .map_err(AppError::from)?;

    Ok(Json(
        json!({"message": "Cart item updated successfully", "quantity": body.quantity}),
    ))
}

#[utoipa::path(
    delete, path = "/api/cart/{user_id}/{item_id}", tag = "Cart",
    params(
        ("user_id" = Uuid, Path, description = "User ID that owns the cart item"),
        ("item_id" = Uuid, Path, description = "Cart item ID"),
    ),
    responses(
        (status = 200, description = "Cart item removed", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin only"),
        (status = 404, description = "Cart item not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn remove_from_cart(
    Path((user_id, item_id)): Path<(Uuid, Uuid)>,
    claims: Claims,
    State(pool): State<DbPool>,
) -> Result<Json<serde_json::Value>, AppError> {
    check_cart_ownership(&claims, user_id)?;

    let result = sqlx::query("DELETE FROM cart_items WHERE id = $1 AND user_id = $2")
        .bind(item_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(AppError::from)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Cart item not found".to_string()));
    }

    Ok(Json(json!({"message": "Cart item removed successfully"})))
}
