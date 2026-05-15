use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use uuid::Uuid;

use crate::auth::AdminClaims;
use crate::error::AppError;
use crate::models::{CreateProduct, Product, ProductResponse};
use crate::services::DbPool;

/// GET /api/products — Public
pub async fn get_products(
    State(pool): State<DbPool>,
) -> Result<Json<Vec<ProductResponse>>, AppError> {
    let products =
        sqlx::query_as::<_, Product>("SELECT * FROM products ORDER BY created_at DESC")
            .fetch_all(&pool)
            .await
            .map_err(AppError::from)?;

    let responses = products.into_iter().map(ProductResponse::from).collect();
    Ok(Json(responses))
}

/// GET /api/products/:id — Public
pub async fn get_product(
    Path(product_id): Path<Uuid>,
    State(pool): State<DbPool>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    Ok(Json(ProductResponse::from(product)))
}

/// POST /api/products — Admin only
pub async fn create_product(
    AdminClaims(_): AdminClaims,
    State(pool): State<DbPool>,
    Json(body): Json<CreateProduct>,
) -> Result<(StatusCode, Json<ProductResponse>), AppError> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        "INSERT INTO products (id, name, price, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(&body.name)
    .bind(body.price)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .map_err(AppError::from)?;

    let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .map_err(AppError::from)?;

    Ok((StatusCode::CREATED, Json(ProductResponse::from(product))))
}
