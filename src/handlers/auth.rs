use axum::{extract::State, http::StatusCode, response::Json};
use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::claims::{
    decode_refresh_token, generate_access_token, generate_refresh_token,
};
use crate::auth::{Claims, JwtKeys, Role};
use crate::error::AppError;
use crate::models::{
    AuthResponse, ForgotPasswordRequest, LoginRequest, RefreshRequest, ResetPasswordRequest,
    SignupRequest, User, UserResponse,
};
use crate::services::DbPool;

fn build_auth_response(
    user_id: Uuid,
    email: &str,
    role: Role,
    keys: &JwtKeys,
) -> Result<AuthResponse, AppError> {
    let access_token = generate_access_token(user_id, email, role.clone(), keys)?;
    let refresh_token = generate_refresh_token(user_id, email, role, keys)?;
    Ok(AuthResponse::new(access_token, refresh_token))
}

pub async fn signup(
    State(pool): State<DbPool>,
    State(keys): State<Arc<JwtKeys>>,
    Json(body): Json<SignupRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    // Check for duplicate email
    let existing = sqlx::query("SELECT id FROM users WHERE email = $1")
        .bind(&body.email)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?;

    if existing.is_some() {
        return Err(AppError::Conflict("Email already in use".to_string()));
    }

    let password_hash = bcrypt::hash(&body.password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::InternalServerError(format!("Password hashing failed: {e}")))?;

    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO users (id, email, name, password_hash, role, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'customer', TRUE, $5, $6)
        "#,
    )
    .bind(id)
    .bind(&body.email)
    .bind(&body.name)
    .bind(&password_hash)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .map_err(AppError::from)?;

    let response = build_auth_response(id, &body.email, Role::Customer, &keys)?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn login(
    State(pool): State<DbPool>,
    State(keys): State<Arc<JwtKeys>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Always return the same error whether email not found or password wrong
    // (prevents email enumeration)
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&body.email)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::Unauthorized)?;

    let password_matches = bcrypt::verify(&body.password, &user.password_hash)
        .map_err(|e| AppError::InternalServerError(format!("Password verification failed: {e}")))?;

    if !password_matches {
        return Err(AppError::Unauthorized);
    }

    if !user.is_active {
        return Err(AppError::Forbidden);
    }

    let response = build_auth_response(user.id, &user.email, user.role, &keys)?;
    Ok(Json(response))
}

pub async fn refresh_token(
    State(pool): State<DbPool>,
    State(keys): State<Arc<JwtKeys>>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let claims = decode_refresh_token(&body.refresh_token, &keys)?;

    let user_id = claims.user_id()?;

    // Confirm user still exists and is active
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::Unauthorized)?;

    if !user.is_active {
        return Err(AppError::Forbidden);
    }

    let response = build_auth_response(user.id, &user.email, user.role, &keys)?;
    Ok(Json(response))
}

pub async fn forgot_password(
    State(pool): State<DbPool>,
    Json(body): Json<ForgotPasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Always return 200 to prevent email enumeration
    let generic_ok = serde_json::json!({
        "message": "If that email is registered, a reset link has been sent."
    });

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&body.email)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?;

    let Some(user) = user else {
        return Ok(Json(generic_ok));
    };

    // Generate a cryptographically random 32-byte hex token
    let token_bytes: [u8; 32] = rand::random();
    let reset_token = hex::encode(token_bytes);
    let expires_at = Utc::now() + Duration::hours(1);

    sqlx::query(
        "UPDATE users SET reset_token = $1, reset_token_expires_at = $2 WHERE id = $3",
    )
    .bind(&reset_token)
    .bind(expires_at)
    .bind(user.id)
    .execute(&pool)
    .await
    .map_err(AppError::from)?;

    // TODO: send email with reset_token
    tracing::debug!(
        user_id = %user.id,
        "Password reset token generated (email sending not yet implemented)"
    );

    Ok(Json(generic_ok))
}

pub async fn reset_password(
    State(pool): State<DbPool>,
    _claims: Claims, // Requires a valid access token
    Json(body): Json<ResetPasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE reset_token = $1",
    )
    .bind(&body.reset_token)
    .fetch_optional(&pool)
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::BadRequest("Invalid or expired reset token".to_string()))?;

    // Check token expiry
    let expires_at = user
        .reset_token_expires_at
        .ok_or_else(|| AppError::BadRequest("Invalid or expired reset token".to_string()))?;

    if Utc::now() > expires_at {
        return Err(AppError::BadRequest("Reset token has expired".to_string()));
    }

    let new_hash = bcrypt::hash(&body.new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::InternalServerError(format!("Password hashing failed: {e}")))?;

    let now = Utc::now();

    sqlx::query(
        r#"UPDATE users
           SET password_hash = $1,
               reset_token = NULL,
               reset_token_expires_at = NULL,
               updated_at = $2
           WHERE id = $3"#,
    )
    .bind(&new_hash)
    .bind(now)
    .bind(user.id)
    .execute(&pool)
    .await
    .map_err(AppError::from)?;

    Ok(Json(serde_json::json!({"message": "Password reset successfully"})))
}

pub async fn me(
    claims: Claims,
    State(pool): State<DbPool>,
) -> Result<Json<UserResponse>, AppError> {
    let user_id = claims.user_id()?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(UserResponse::from(user)))
}
