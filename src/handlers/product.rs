use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::auth::AdminClaims;
use crate::error::AppError;
use crate::models::{CreateProduct, Product, ProductFilterParams, ProductResponse, UpdateProduct};
use crate::services::DbPool;

#[utoipa::path(
    get, path = "/api/products", tag = "Products",
    params(
        ("category" = Option<String>, Query, description = "Filter by category"),
        ("min_price" = Option<i32>, Query, description = "Minimum price in kobo"),
        ("max_price" = Option<i32>, Query, description = "Maximum price in kobo"),
        ("search" = Option<String>, Query, description = "Search in name and description"),
        ("in_stock" = Option<bool>, Query, description = "Filter by stock availability"),
    ),
    responses(
        (status = 200, description = "List of active products", body = Vec<ProductResponse>),
    )
)]
pub async fn get_products(
    State(pool): State<DbPool>,
    Query(filters): Query<ProductFilterParams>,
) -> Result<Json<Vec<ProductResponse>>, AppError> {
    // Build WHERE clauses dynamically
    let conditions = vec!["is_active = TRUE"];
    let param_idx = 1usize;

    // We use a fixed query approach to avoid sqlx QueryBuilder complexity at this stage
    // All active products, then filter in Rust (acceptable for small catalogs)
    // Production-scale would use QueryBuilder — see Phase 5 improvements
    let products = sqlx::query_as::<_, Product>(
        "SELECT * FROM products WHERE is_active = TRUE ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(AppError::from)?;

    // Apply filters in memory
    let responses: Vec<ProductResponse> = products
        .into_iter()
        .filter(|p| {
            if let Some(ref cat) = filters.category {
                if p.category.as_deref() != Some(cat.as_str()) {
                    return false;
                }
            }
            if let Some(min) = filters.min_price {
                if p.price < min {
                    return false;
                }
            }
            if let Some(max) = filters.max_price {
                if p.price > max {
                    return false;
                }
            }
            if let Some(ref search) = filters.search {
                let s = search.to_lowercase();
                if !p.name.to_lowercase().contains(&s)
                    && !p.description.as_deref().unwrap_or("").to_lowercase().contains(&s)
                {
                    return false;
                }
            }
            if let Some(in_stock) = filters.in_stock {
                if in_stock && p.stock_quantity <= 0 {
                    return false;
                }
                if !in_stock && p.stock_quantity > 0 {
                    return false;
                }
            }
            true
        })
        .map(ProductResponse::from)
        .collect();

    // suppress unused variable warning
    let _ = (conditions, param_idx);

    Ok(Json(responses))
}

#[utoipa::path(
    get, path = "/api/products/{id}", tag = "Products",
    params(("id" = Uuid, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Product details", body = ProductResponse),
        (status = 404, description = "Product not found"),
    )
)]
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

#[utoipa::path(
    post, path = "/api/products", tag = "Products",
    request_body = CreateProduct,
    responses(
        (status = 201, description = "Product created", body = ProductResponse),
        (status = 400, description = "Invalid price"),
        (status = 403, description = "Admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_product(
    _admin: AdminClaims,
    State(pool): State<DbPool>,
    Json(body): Json<CreateProduct>,
) -> Result<(StatusCode, Json<ProductResponse>), AppError> {
    if body.price < 1 {
        return Err(AppError::BadRequest(
            "Price must be at least 1 kobo".to_string(),
        ));
    }

    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let stock = body.stock_quantity.unwrap_or(0);

    sqlx::query(
        r#"INSERT INTO products
           (id, name, description, price, stock_quantity, category, image_url, is_active, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE, $8, $9)"#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(body.price)
    .bind(stock)
    .bind(&body.category)
    .bind(&body.image_url)
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

#[utoipa::path(
    put, path = "/api/products/{id}", tag = "Products",
    params(("id" = Uuid, Path, description = "Product ID")),
    request_body = UpdateProduct,
    responses(
        (status = 200, description = "Product updated", body = ProductResponse),
        (status = 404, description = "Product not found"),
        (status = 403, description = "Admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_product(
    Path(product_id): Path<Uuid>,
    _admin: AdminClaims,
    State(pool): State<DbPool>,
    Json(body): Json<UpdateProduct>,
) -> Result<Json<ProductResponse>, AppError> {
    // Verify product exists
    let existing = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    let now = chrono::Utc::now();
    let name = body.name.unwrap_or(existing.name);
    let description = body.description.or(existing.description);
    let price = body.price.unwrap_or(existing.price);
    let stock_quantity = body.stock_quantity.unwrap_or(existing.stock_quantity);
    let category = body.category.or(existing.category);
    let image_url = body.image_url.or(existing.image_url);
    let is_active = body.is_active.unwrap_or(existing.is_active);

    if price < 1 {
        return Err(AppError::BadRequest(
            "Price must be at least 1 kobo".to_string(),
        ));
    }

    sqlx::query(
        r#"UPDATE products
           SET name = $1, description = $2, price = $3, stock_quantity = $4,
               category = $5, image_url = $6, is_active = $7, updated_at = $8
           WHERE id = $9"#,
    )
    .bind(&name)
    .bind(&description)
    .bind(price)
    .bind(stock_quantity)
    .bind(&category)
    .bind(&image_url)
    .bind(is_active)
    .bind(now)
    .bind(product_id)
    .execute(&pool)
    .await
    .map_err(AppError::from)?;

    let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_one(&pool)
        .await
        .map_err(AppError::from)?;

    Ok(Json(ProductResponse::from(product)))
}

#[utoipa::path(
    delete, path = "/api/products/{id}", tag = "Products",
    params(("id" = Uuid, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Product deactivated"),
        (status = 404, description = "Product not found"),
        (status = 403, description = "Admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_product(
    Path(product_id): Path<Uuid>,
    _admin: AdminClaims,
    State(pool): State<DbPool>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query(
        "UPDATE products SET is_active = FALSE, updated_at = NOW() WHERE id = $1 AND is_active = TRUE",
    )
    .bind(product_id)
    .execute(&pool)
    .await
    .map_err(AppError::from)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Product not found".to_string()));
    }

    Ok(Json(json!({"message": "Product deactivated successfully"})))
}
