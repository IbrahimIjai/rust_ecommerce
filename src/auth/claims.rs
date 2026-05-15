use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::keys::JwtKeys;
use crate::error::AppError;
use crate::services::AppState;

pub const ACCESS_TOKEN_MINUTES: i64 = 15;
pub const REFRESH_TOKEN_DAYS: i64 = 7;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum Role {
    Customer,
    Admin,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: Role,
    pub token_type: TokenType,
    pub exp: usize,
    pub iat: usize,
}

impl Claims {
    pub fn user_id(&self) -> Result<Uuid, AppError> {
        Uuid::parse_str(&self.sub)
            .map_err(|_| AppError::InternalServerError("Invalid user ID in token".to_string()))
    }
}

pub struct AdminClaims(pub Claims);

// ─── FromRequestParts ────────────────────────────────────────────────────────

#[async_trait]
impl FromRequestParts<AppState> for Claims {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::Unauthorized)?;

        let token_data = decode::<Claims>(
            bearer.token(),
            &state.jwt_keys.decoding,
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        if token_data.claims.token_type != TokenType::Access {
            return Err(AppError::Unauthorized);
        }

        Ok(token_data.claims)
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AdminClaims {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = Claims::from_request_parts(parts, state).await?;

        if claims.role != Role::Admin {
            return Err(AppError::Forbidden);
        }

        Ok(AdminClaims(claims))
    }
}

// ─── Token generation helpers ───────────────────────────────────────────────

pub fn generate_access_token(
    user_id: Uuid,
    email: &str,
    role: Role,
    keys: &JwtKeys,
) -> Result<String, AppError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        role,
        token_type: TokenType::Access,
        exp: (now + Duration::minutes(ACCESS_TOKEN_MINUTES)).timestamp() as usize,
        iat: now.timestamp() as usize,
    };
    encode(&Header::default(), &claims, &keys.encoding)
        .map_err(|e| AppError::InternalServerError(format!("Token encoding failed: {e}")))
}

pub fn generate_refresh_token(
    user_id: Uuid,
    email: &str,
    role: Role,
    keys: &JwtKeys,
) -> Result<String, AppError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        role,
        token_type: TokenType::Refresh,
        exp: (now + Duration::days(REFRESH_TOKEN_DAYS)).timestamp() as usize,
        iat: now.timestamp() as usize,
    };
    encode(&Header::default(), &claims, &keys.encoding)
        .map_err(|e| AppError::InternalServerError(format!("Token encoding failed: {e}")))
}

pub fn decode_refresh_token(token: &str, keys: &JwtKeys) -> Result<Claims, AppError> {
    let data = decode::<Claims>(token, &keys.decoding, &Validation::default())
        .map_err(|_| AppError::Unauthorized)?;

    if data.claims.token_type != TokenType::Refresh {
        return Err(AppError::Unauthorized);
    }

    Ok(data.claims)
}
