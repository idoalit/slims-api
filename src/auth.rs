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
use sqlx::{FromRow, QueryBuilder};
use utoipa::ToSchema;

use crate::{
    config::AppState,
    error::AppError,
    jsonapi::{JsonApiDocument, resource, single_document},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Librarian,
    Staff,
    Member,
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub role: Role,
    #[serde(default)]
    pub access: Vec<ModulePermission>,
    pub exp: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ToSchema)]
pub enum Permission {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ToSchema)]
#[allow(dead_code)]
#[repr(i64)]
pub enum ModuleAccess {
    Bibliography = 1,
    Circulation = 2,
    Membership = 3,
    MasterFile = 4,
    StockTake = 5,
    System = 6,
    Reporting = 7,
    SerialControl = 8,
}

impl ModuleAccess {
    pub fn id(self) -> i64 {
        self as i64
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ModulePermission {
    pub module_id: i64,
    pub read: bool,
    pub write: bool,
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
    // Map based on group membership: group_id 1 or user_type==1 => Admin, else Staff
    let group_ids = parse_groups(user.groups.as_deref());
    if group_ids.contains(&1) {
        return Role::Admin;
    }
    if matches!(user.user_type, Some(1)) {
        Role::Admin
    } else {
        Role::Staff
    }
}

impl AuthUser {
    pub fn require_access(
        &self,
        module: ModuleAccess,
        permission: Permission,
    ) -> Result<(), AppError> {
        let module_id = module.id();
        let can_access = self.claims.access.iter().find(|a| a.module_id == module_id);

        let allowed = match (can_access, permission) {
            (Some(access), Permission::Read) => access.read || access.write,
            (Some(access), Permission::Write) => access.write,
            _ => false,
        };

        if allowed {
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub expires_at: usize,
    pub role: Role,
    pub access: Vec<ModulePermission>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub passwd: String,
    pub groups: Option<String>,
    pub user_type: Option<i16>,
}

#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login success", body = JsonApiDocument),
        (status = 401, description = "Invalid credentials"),
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<JsonApiDocument>, AppError> {
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

    let group_ids = parse_groups(user.groups.as_deref());
    let access = fetch_group_access(&state, &group_ids).await?;
    let exp = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        + Duration::from_secs(60 * 60))
    .as_secs() as usize;

    let claims = Claims {
        sub: user.user_id,
        username: user.username,
        role: role.clone(),
        access: access.clone(),
        exp,
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )?;

    let response = AuthResponse {
        token,
        expires_at: exp,
        role,
        access,
    };

    let token_id = response.token.clone();
    Ok(Json(single_document(resource(
        "tokens",
        token_id,
        response,
    ))))
}

pub fn extract_secret(secret: String) -> Arc<str> {
    Arc::from(secret.into_boxed_str())
}

fn parse_groups(raw: Option<&str>) -> Vec<i64> {
    let Some(raw) = raw else {
        return Vec::new();
    };

    raw.split('"')
        .enumerate()
        .filter_map(|(idx, part)| (idx % 2 == 1).then(|| part))
        .filter_map(|part| part.trim().parse::<i64>().ok())
        .collect()
}

#[derive(Debug, FromRow)]
struct GroupAccessRow {
    module_id: i64,
    r: i32,
    w: i32,
}

async fn fetch_group_access(
    state: &AppState,
    group_ids: &[i64],
) -> Result<Vec<ModulePermission>, AppError> {
    if group_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = QueryBuilder::new(
        "SELECT module_id, MAX(r) AS r, MAX(w) AS w FROM group_access WHERE group_id IN (",
    );

    let mut separated = builder.separated(",");
    for group_id in group_ids {
        separated.push_bind(group_id);
    }
    builder.push(") GROUP BY module_id");

    let rows = builder
        .build_query_as::<GroupAccessRow>()
        .fetch_all(&state.pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| ModulePermission {
            module_id: row.module_id,
            read: row.r != 0,
            write: row.w != 0,
        })
        .collect())
}
