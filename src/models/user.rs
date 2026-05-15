use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use validator::Validate;

use crate::auth::Role;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub role: Role,
    pub is_active: bool,
    pub reset_token: Option<String>,
    pub reset_token_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ─── Request bodies ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SignupRequest {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 2, max = 100, message = "must be 2–100 characters"))]
    pub name: String,
    #[validate(length(min = 8, message = "must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 1, message = "required"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordValidated {
    #[validate(length(min = 1, message = "required"))]
    pub reset_token: String,
    #[validate(length(min = 8, message = "must be at least 8 characters"))]
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub reset_token: String,
    pub new_password: String,
}

// ─── Response bodies ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: Role,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    /// Seconds until access token expires
    pub expires_in: u64,
}

impl AuthResponse {
    pub fn new(access_token: String, refresh_token: String) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: 900, // 15 minutes
        }
    }
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            name: u.name,
            role: u.role,
            is_active: u.is_active,
            created_at: u.created_at,
        }
    }
}
