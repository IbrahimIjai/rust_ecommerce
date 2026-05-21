use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::auth::{AdminClaims, Claims, Role};
use crate::error::AppError;
use crate::models::{CreateUser, User, UserResponse};
use crate::services::DbPool;

#[utoipa::path(
    get, path = "/api/users/", tag = "Users",
    responses(
        (status = 200, description = "All users", body = Vec<UserResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_users(
    _admin: AdminClaims,
    State(pool): State<DbPool>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
        .fetch_all(&pool)
        .await
        .map_err(AppError::from)?;

    let responses = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}

#[utoipa::path(
    post, path = "/api/users/", tag = "Users",
    request_body = CreateUser,
    responses(
        (status = 201, description = "User created", body = UserResponse),
        (status = 409, description = "Email already in use"),
    )
)]
pub async fn create_user(
    State(pool): State<DbPool>,
    Json(body): Json<CreateUser>,
) -> Result<(StatusCode, Json<UserResponse>), AppError> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, created_at, updated_at) VALUES ($1, $2, $3, '', $4, $5)",
    )
    .bind(id)
    .bind(&body.email)
    .bind(&body.name)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
            AppError::Conflict("Email already in use".to_string())
        }
        _ => AppError::from(e),
    })?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .map_err(AppError::from)?;

    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

#[utoipa::path(
    get, path = "/api/users/{id}", tag = "Users",
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User profile", body = UserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Own profile or admin only"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_user(
    Path(user_id): Path<Uuid>,
    claims: Claims,
    State(pool): State<DbPool>,
) -> Result<Json<UserResponse>, AppError> {
    // Customers can only fetch their own profile
    if claims.role != Role::Admin && claims.user_id()? != user_id {
        return Err(AppError::Forbidden);
    }

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(UserResponse::from(user)))
}

#[utoipa::path(
    delete, path = "/api/users/{id}", tag = "Users",
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User deleted", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin only"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_user(
    Path(user_id): Path<Uuid>,
    _admin: AdminClaims,
    State(pool): State<DbPool>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(AppError::from)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    Ok(Json(json!({"message": "User deleted successfully"})))
}
