use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, async_trait,
    extract::{FromRequestParts, State},
    http::{HeaderMap, header, request::Parts},
};
use bcrypt::verify;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{config::AppState, error::AppError};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Librarian,
    Staff,
    Member,
}

impl Role {
    pub fn matches_any(&self, roles: &[Role]) -> bool {
        roles.iter().any(|r| r == self)
    }
}

impl TryFrom<String> for Role {
    type Error = AppError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "admin" => Ok(Role::Admin),
            "librarian" => Ok(Role::Librarian),
            "staff" => Ok(Role::Staff),
            "member" => Ok(Role::Member),
            other => Err(AppError::BadRequest(format!("invalid role: {other}"))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub role: Role,
    pub exp: usize,
}

pub struct AuthUser {
    pub claims: Claims,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(&parts.headers)?;
        let decoding_key = DecodingKey::from_secret(state.jwt_secret.as_bytes());
        let token_data =
            decode::<Claims>(&token, &decoding_key, &Validation::new(Algorithm::HS256))?;

        Ok(AuthUser {
            claims: token_data.claims,
        })
    }
}

fn user_to_role(user: &User) -> Role {
    // Simplified mapping: Administrator group or user_type==1 => Admin, else Staff
    if let Some(groups) = &user.groups {
        if groups.contains("1") {
            return Role::Admin;
        }
    }
    if matches!(user.user_type, Some(1)) {
        Role::Admin
    } else {
        Role::Staff
    }
}

impl AuthUser {
    pub fn require_roles(&self, allowed: &[Role]) -> Result<(), AppError> {
        if self.claims.role.matches_any(allowed) {
            Ok(())
        } else {
            Err(AppError::Forbidden("insufficient permissions".into()))
        }
    }
}

fn extract_bearer(headers: &HeaderMap) -> Result<String, AppError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid Authorization header".into()))?;

    if let Some(token) = auth_str.strip_prefix("Bearer ") {
        Ok(token.to_string())
    } else {
        Err(AppError::Unauthorized(
            "Authorization header must be Bearer".into(),
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub expires_at: usize,
    pub role: Role,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub passwd: String,
    pub groups: Option<String>,
    pub user_type: Option<i16>,
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT user_id, username, passwd, `groups`, user_type FROM `user` WHERE username = ?",
    )
    .bind(&payload.username)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("invalid credentials".into()))?;

    verify(&payload.password, &user.passwd)
        .map_err(|_| AppError::Unauthorized("invalid credentials".into()))
        .and_then(|ok| {
            if ok {
                Ok(())
            } else {
                Err(AppError::Unauthorized("invalid credentials".into()))
            }
        })?;

    let role = user_to_role(&user);
    let exp = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        + Duration::from_secs(60 * 60))
    .as_secs() as usize;

    let claims = Claims {
        sub: user.user_id,
        username: user.username,
        role: role.clone(),
        exp,
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )?;

    Ok(Json(AuthResponse {
        token,
        expires_at: exp,
        role,
    }))
}

pub fn extract_secret(secret: String) -> Arc<str> {
    Arc::from(secret.into_boxed_str())
}
