use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, async_trait,
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bcrypt::verify;
use chrono::{Duration as ChronoDuration, NaiveDateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, MySql, QueryBuilder, Transaction};
use subtle::ConstantTimeEq;
use utoipa::ToSchema;

use crate::{
    config::AppState,
    error::AppError,
    jsonapi::{JsonApiDocument, resource, single_document},
};

const REFRESH_COOKIE_SECURE: &str = "__Host-slims-refresh";
const REFRESH_COOKIE_INSECURE: &str = "slims_refresh";
const AJAX_HEADER: &str = "x-requested-with";
const LOGIN_WINDOW_MINUTES: i64 = 15;
const LOGIN_MAX_ATTEMPTS: u32 = 5;
const ROTATION_GRACE_SECONDS: i64 = 2;
const DUMMY_PASSWORD_HASH: &str = "$2y$12$qslOSX2gat/z1G3/DeVsleTVS4fVJE28dNkp2QE88mFYdhIMWSbD6";

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
    pub sub: String,
    pub username: String,
    pub role: Role,
    #[serde(default)]
    pub access: Vec<ModulePermission>,
    pub exp: usize,
    pub iat: usize,
    pub nbf: usize,
    pub iss: String,
    pub aud: String,
    pub jti: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ToSchema)]
pub enum Permission {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ToSchema)]
#[allow(dead_code)]
pub enum ModuleAccess {
    Bibliography,
    Circulation,
    Membership,
    MasterFile,
    StockTake,
    System,
    Reporting,
    SerialControl,
}

impl ModuleAccess {
    pub fn name(self) -> &'static str {
        match self {
            Self::Bibliography => "bibliography",
            Self::Circulation => "circulation",
            Self::Membership => "membership",
            Self::MasterFile => "master_file",
            Self::StockTake => "stock_take",
            Self::System => "system",
            Self::Reporting => "reporting",
            Self::SerialControl => "serial_control",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ModulePermission {
    pub module_id: i64,
    pub module_name: String,
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
        let token_data = decode_access_token(state, &token)?;

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
    pub fn can_access(&self, module: ModuleAccess, permission: Permission) -> bool {
        let access = self
            .claims
            .access
            .iter()
            .find(|item| item.module_name == module.name());
        match (access, permission) {
            (Some(access), Permission::Read) => access.read || access.write,
            (Some(access), Permission::Write) => access.write,
            _ => false,
        }
    }

    pub fn require_access(
        &self,
        module: ModuleAccess,
        permission: Permission,
    ) -> Result<(), AppError> {
        if self.can_access(module, permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden("insufficient permissions".into()))
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CurrentUserResponse {
    pub username: String,
    pub role: Role,
    pub access: Vec<ModulePermission>,
    pub expires_at: usize,
}

#[utoipa::path(
    get,
    path = "/auth/me",
    responses((status = 200, body = JsonApiDocument), (status = 401)),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
pub async fn me(auth: AuthUser) -> Json<JsonApiDocument> {
    let claims = auth.claims;
    Json(single_document(resource(
        "users",
        claims.sub,
        CurrentUserResponse {
            username: claims.username,
            role: claims.role,
            access: claims.access,
            expires_at: claims.exp,
        },
    )))
}

pub fn extract_bearer(headers: &HeaderMap) -> Result<String, AppError> {
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
    #[serde(default)]
    pub remember_me: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub expires_at: usize,
    pub role: Role,
    pub access: Vec<ModulePermission>,
}

struct IssuedAccessToken {
    response: AuthResponse,
    token_id: String,
}

struct IssuedRefreshToken {
    raw: String,
    expires_at: NaiveDateTime,
    remember_me: bool,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub passwd: String,
    pub groups: Option<String>,
    pub user_type: Option<i16>,
}

#[derive(Debug, FromRow)]
struct RefreshSession {
    selector: String,
    validator_hash: String,
    family_id: String,
    user_id: i64,
    remember_me: bool,
    expires_at: NaiveDateTime,
    last_used_at: Option<NaiveDateTime>,
    revoked_at: Option<NaiveDateTime>,
    replaced_by: Option<String>,
}

#[derive(Debug, FromRow)]
struct LoginAttempt {
    attempts: u32,
    window_started_at: NaiveDateTime,
    locked_until: Option<NaiveDateTime>,
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
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, AppError> {
    let username = payload.username.trim();
    if username.is_empty() || payload.password.is_empty() {
        return Err(AppError::Unauthorized("invalid credentials".into()));
    }

    let login_key = hash_value(&username.to_lowercase());
    enforce_login_rate_limit(&state, &login_key).await?;

    let user = sqlx::query_as::<_, User>(
        "SELECT user_id, username, passwd, `groups`, user_type FROM `user` WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(&state.pool)
    .await?;

    let password_hash = user
        .as_ref()
        .map(|user| user.passwd.as_str())
        .unwrap_or(DUMMY_PASSWORD_HASH);
    let password_valid = verify(&payload.password, password_hash).unwrap_or(false);
    if !password_valid || user.is_none() {
        record_failed_login(&state, &login_key).await?;
        return Err(AppError::Unauthorized("invalid credentials".into()));
    }
    let user = user.expect("checked above");
    clear_login_attempts(&state, &login_key).await?;

    let role = user_to_role(&user);
    let group_ids = parse_groups(user.groups.as_deref());
    let access = fetch_group_access(&state, &group_ids).await?;
    revoke_cookie_session_if_present(&state, &headers).await?;
    let access_token = issue_access_token(&state, &user, role, access)?;
    let refresh_token = create_refresh_session(&state, user.user_id, payload.remember_me).await?;
    cleanup_auth_records(&state).await;

    Ok(auth_success_response(&state, access_token, refresh_token))
}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    responses(
        (status = 200, description = "Access token refreshed", body = JsonApiDocument),
        (status = 401, description = "Refresh session invalid or expired"),
        (status = 403, description = "Missing CSRF protection header"),
    ),
    tag = "Auth"
)]
pub async fn refresh(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let result = refresh_session(&state, &headers).await;
    match result {
        Ok((access_token, refresh_token)) => {
            auth_success_response(&state, access_token, refresh_token)
        }
        Err(error) => {
            let clear_cookie = matches!(error, AppError::Unauthorized(_));
            auth_error_response(&state, error, clear_cookie)
        }
    }
}

#[utoipa::path(
    post,
    path = "/auth/logout",
    responses((status = 204, description = "Session revoked"), (status = 403)),
    tag = "Auth"
)]
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(error) = verify_ajax_request(&headers) {
        return auth_error_response(&state, error, false);
    }

    if let Err(error) = revoke_cookie_session_if_present(&state, &headers).await {
        return auth_error_response(&state, error, false);
    }

    let mut response = StatusCode::NO_CONTENT.into_response();
    set_security_headers(&mut response, Some(clear_refresh_cookie(&state)));
    response
}

fn issue_access_token(
    state: &AppState,
    user: &User,
    role: Role,
    access: Vec<ModulePermission>,
) -> Result<IssuedAccessToken, AppError> {
    let issued_at = unix_timestamp();
    let expires_at = issued_at.saturating_add(state.access_token_ttl.as_secs() as usize);
    let token_id = random_value(16);
    let claims = Claims {
        sub: user.user_id.to_string(),
        username: user.username.clone(),
        role: role.clone(),
        access: access.clone(),
        exp: expires_at,
        iat: issued_at,
        nbf: issued_at,
        iss: state.jwt_issuer.to_string(),
        aud: state.jwt_audience.to_string(),
        jti: token_id.clone(),
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )?;

    Ok(IssuedAccessToken {
        response: AuthResponse {
            token,
            expires_at,
            role,
            access,
        },
        token_id,
    })
}

fn decode_access_token(
    state: &AppState,
    token: &str,
) -> Result<jsonwebtoken::TokenData<Claims>, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 5;
    validation.set_audience(&[state.jwt_audience.as_ref()]);
    validation.set_issuer(&[state.jwt_issuer.as_ref()]);
    validation.set_required_spec_claims(&["exp", "nbf", "sub", "iss", "aud"]);
    Ok(decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &validation,
    )?)
}

async fn create_refresh_session(
    state: &AppState,
    user_id: i64,
    remember_me: bool,
) -> Result<IssuedRefreshToken, AppError> {
    let selector = random_value(16);
    let validator = random_value(32);
    let now = Utc::now().naive_utc();
    let ttl = if remember_me {
        state.refresh_remember_ttl
    } else {
        state.refresh_session_ttl
    };
    let expires_at = now
        + ChronoDuration::from_std(ttl).map_err(|error| AppError::Internal(error.to_string()))?;

    sqlx::query(
        "INSERT INTO auth_refresh_sessions \
         (selector, validator_hash, family_id, user_id, remember_me, expires_at, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&selector)
    .bind(hash_value(&validator))
    .bind(&selector)
    .bind(user_id)
    .bind(remember_me)
    .bind(expires_at)
    .bind(now)
    .execute(&state.pool)
    .await?;

    Ok(IssuedRefreshToken {
        raw: format!("{selector}.{validator}"),
        expires_at,
        remember_me,
    })
}

async fn refresh_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(IssuedAccessToken, IssuedRefreshToken), AppError> {
    verify_ajax_request(headers)?;
    let raw = refresh_cookie(state, headers)
        .ok_or_else(|| AppError::Unauthorized("refresh session missing".into()))?;
    let (selector, validator) = parse_refresh_token(&raw)?;
    let validator_hash = hash_value(&validator);
    let now = Utc::now().naive_utc();
    let mut transaction = state.pool.begin().await?;

    let session = sqlx::query_as::<_, RefreshSession>(
        "SELECT selector, validator_hash, family_id, user_id, remember_me, expires_at, \
         last_used_at, revoked_at, replaced_by \
         FROM auth_refresh_sessions WHERE selector = ? FOR UPDATE",
    )
    .bind(&selector)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| AppError::Unauthorized("refresh session invalid".into()))?;

    let validator_matches: bool = session
        .validator_hash
        .as_bytes()
        .ct_eq(validator_hash.as_bytes())
        .into();
    if session.revoked_at.is_some() {
        let just_rotated = validator_matches
            && session.replaced_by.is_some()
            && session.last_used_at.is_some_and(|used_at| {
                used_at > now - ChronoDuration::seconds(ROTATION_GRACE_SECONDS)
            });
        if just_rotated {
            return Err(AppError::Conflict(
                "refresh session was just rotated".into(),
            ));
        }
        revoke_family_in_transaction(&mut transaction, &session.family_id, now).await?;
        transaction.commit().await?;
        return Err(AppError::Unauthorized(
            "refresh token reuse detected".into(),
        ));
    }
    if !validator_matches {
        revoke_family_in_transaction(&mut transaction, &session.family_id, now).await?;
        transaction.commit().await?;
        return Err(AppError::Unauthorized("refresh token invalid".into()));
    }
    if session.expires_at <= now {
        revoke_family_in_transaction(&mut transaction, &session.family_id, now).await?;
        transaction.commit().await?;
        return Err(AppError::Unauthorized("refresh session expired".into()));
    }

    let user = match fetch_user_by_id(state, session.user_id).await? {
        Some(user) => user,
        None => {
            revoke_family_in_transaction(&mut transaction, &session.family_id, now).await?;
            transaction.commit().await?;
            return Err(AppError::Unauthorized("user no longer exists".into()));
        }
    };
    let role = user_to_role(&user);
    let access = fetch_group_access(state, &parse_groups(user.groups.as_deref())).await?;
    let access_token = issue_access_token(state, &user, role, access)?;

    let next_selector = random_value(16);
    let next_validator = random_value(32);
    sqlx::query(
        "UPDATE auth_refresh_sessions \
         SET revoked_at = ?, last_used_at = ?, replaced_by = ? WHERE selector = ?",
    )
    .bind(now)
    .bind(now)
    .bind(&next_selector)
    .bind(&session.selector)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO auth_refresh_sessions \
         (selector, validator_hash, family_id, user_id, remember_me, expires_at, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&next_selector)
    .bind(hash_value(&next_validator))
    .bind(&session.family_id)
    .bind(session.user_id)
    .bind(session.remember_me)
    .bind(session.expires_at)
    .bind(now)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    let refresh_token = IssuedRefreshToken {
        raw: format!("{next_selector}.{next_validator}"),
        expires_at: session.expires_at,
        remember_me: session.remember_me,
    };
    cleanup_auth_records(state).await;
    Ok((access_token, refresh_token))
}

async fn fetch_user_by_id(state: &AppState, user_id: i64) -> Result<Option<User>, AppError> {
    Ok(sqlx::query_as::<_, User>(
        "SELECT user_id, username, passwd, `groups`, user_type FROM `user` WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?)
}

async fn revoke_refresh_family(
    state: &AppState,
    selector: &str,
    validator: &str,
) -> Result<(), AppError> {
    let mut transaction = state.pool.begin().await?;
    let session = sqlx::query_as::<_, RefreshSession>(
        "SELECT selector, validator_hash, family_id, user_id, remember_me, expires_at, \
         last_used_at, revoked_at, replaced_by \
         FROM auth_refresh_sessions WHERE selector = ? FOR UPDATE",
    )
    .bind(selector)
    .fetch_optional(&mut *transaction)
    .await?;

    if let Some(session) = session {
        let supplied_hash = hash_value(validator);
        let matches: bool = session
            .validator_hash
            .as_bytes()
            .ct_eq(supplied_hash.as_bytes())
            .into();
        if matches || session.revoked_at.is_some() {
            revoke_family_in_transaction(
                &mut transaction,
                &session.family_id,
                Utc::now().naive_utc(),
            )
            .await?;
        }
    }
    transaction.commit().await?;
    Ok(())
}

async fn revoke_cookie_session_if_present(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    if let Some(raw) = refresh_cookie(state, headers)
        && let Ok((selector, validator)) = parse_refresh_token(&raw)
    {
        revoke_refresh_family(state, &selector, &validator).await?;
    }
    Ok(())
}

async fn revoke_family_in_transaction(
    transaction: &mut Transaction<'_, MySql>,
    family_id: &str,
    revoked_at: NaiveDateTime,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE auth_refresh_sessions SET revoked_at = COALESCE(revoked_at, ?) WHERE family_id = ?",
    )
    .bind(revoked_at)
    .bind(family_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn enforce_login_rate_limit(state: &AppState, login_key: &str) -> Result<(), AppError> {
    let attempt = sqlx::query_as::<_, LoginAttempt>(
        "SELECT attempts, window_started_at, locked_until \
         FROM auth_login_attempts WHERE username_hash = ?",
    )
    .bind(login_key)
    .fetch_optional(&state.pool)
    .await?;

    if attempt
        .and_then(|attempt| attempt.locked_until)
        .is_some_and(|locked_until| locked_until > Utc::now().naive_utc())
    {
        return Err(AppError::TooManyRequests(
            "terlalu banyak percobaan login; coba lagi nanti".into(),
        ));
    }
    Ok(())
}

async fn record_failed_login(state: &AppState, login_key: &str) -> Result<(), AppError> {
    let now = Utc::now().naive_utc();
    let window_cutoff = now - ChronoDuration::minutes(LOGIN_WINDOW_MINUTES);
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "INSERT IGNORE INTO auth_login_attempts \
         (username_hash, attempts, window_started_at, locked_until) VALUES (?, 0, ?, NULL)",
    )
    .bind(login_key)
    .bind(now)
    .execute(&mut *transaction)
    .await?;
    let existing = sqlx::query_as::<_, LoginAttempt>(
        "SELECT attempts, window_started_at, locked_until \
         FROM auth_login_attempts WHERE username_hash = ? FOR UPDATE",
    )
    .bind(login_key)
    .fetch_optional(&mut *transaction)
    .await?;

    let (attempts, window_started_at) = match existing {
        Some(existing) if existing.window_started_at >= window_cutoff => (
            existing.attempts.saturating_add(1),
            existing.window_started_at,
        ),
        _ => (1, now),
    };
    let locked_until = (attempts >= LOGIN_MAX_ATTEMPTS)
        .then_some(now + ChronoDuration::minutes(LOGIN_WINDOW_MINUTES));

    sqlx::query(
        "INSERT INTO auth_login_attempts \
         (username_hash, attempts, window_started_at, locked_until) VALUES (?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE attempts = VALUES(attempts), \
         window_started_at = VALUES(window_started_at), locked_until = VALUES(locked_until)",
    )
    .bind(login_key)
    .bind(attempts)
    .bind(window_started_at)
    .bind(locked_until)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn clear_login_attempts(state: &AppState, login_key: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM auth_login_attempts WHERE username_hash = ?")
        .bind(login_key)
        .execute(&state.pool)
        .await?;
    Ok(())
}

async fn cleanup_auth_records(state: &AppState) {
    let refresh_cutoff = Utc::now().naive_utc() - ChronoDuration::days(7);
    let login_cutoff = Utc::now().naive_utc() - ChronoDuration::days(1);
    let _ = sqlx::query(
        "DELETE FROM auth_refresh_sessions \
         WHERE expires_at < ? OR (revoked_at IS NOT NULL AND revoked_at < ?)",
    )
    .bind(Utc::now().naive_utc())
    .bind(refresh_cutoff)
    .execute(&state.pool)
    .await;
    let _ = sqlx::query(
        "DELETE FROM auth_login_attempts WHERE window_started_at < ? \
         AND (locked_until IS NULL OR locked_until < ?)",
    )
    .bind(login_cutoff)
    .bind(Utc::now().naive_utc())
    .execute(&state.pool)
    .await;
}

fn auth_success_response(
    state: &AppState,
    access_token: IssuedAccessToken,
    refresh_token: IssuedRefreshToken,
) -> Response {
    let document = single_document(resource(
        "tokens",
        access_token.token_id,
        access_token.response,
    ));
    let mut response = Json(document).into_response();
    set_security_headers(
        &mut response,
        Some(build_refresh_cookie(state, &refresh_token)),
    );
    response
}

fn auth_error_response(state: &AppState, error: AppError, clear_cookie: bool) -> Response {
    let mut response = error.into_response();
    set_security_headers(
        &mut response,
        clear_cookie.then(|| clear_refresh_cookie(state)),
    );
    response
}

fn set_security_headers(response: &mut Response, cookie: Option<String>) {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    if let Some(cookie) = cookie.and_then(|value| HeaderValue::from_str(&value).ok()) {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
}

fn build_refresh_cookie(state: &AppState, token: &IssuedRefreshToken) -> String {
    let mut cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict",
        refresh_cookie_name(state),
        token.raw
    );
    if state.cookie_secure {
        cookie.push_str("; Secure");
    }
    if token.remember_me {
        let remaining = (token.expires_at - Utc::now().naive_utc())
            .num_seconds()
            .max(0);
        cookie.push_str(&format!("; Max-Age={remaining}"));
    }
    cookie
}

fn clear_refresh_cookie(state: &AppState) -> String {
    let mut cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        refresh_cookie_name(state)
    );
    if state.cookie_secure {
        cookie.push_str("; Secure");
    }
    cookie
}

fn refresh_cookie(state: &AppState, headers: &HeaderMap) -> Option<String> {
    let expected_name = refresh_cookie_name(state);
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find_map(|(name, value)| (name == expected_name).then(|| value.to_string()))
}

fn refresh_cookie_name(state: &AppState) -> &'static str {
    if state.cookie_secure {
        REFRESH_COOKIE_SECURE
    } else {
        REFRESH_COOKIE_INSECURE
    }
}

fn verify_ajax_request(headers: &HeaderMap) -> Result<(), AppError> {
    let valid = headers
        .get(AJAX_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("XMLHttpRequest"));
    if valid {
        Ok(())
    } else {
        Err(AppError::Forbidden("missing CSRF protection header".into()))
    }
}

fn parse_refresh_token(raw: &str) -> Result<(String, String), AppError> {
    let (selector, validator) = raw
        .split_once('.')
        .ok_or_else(|| AppError::Unauthorized("refresh session invalid".into()))?;
    if selector.len() != 22 || validator.len() != 43 {
        return Err(AppError::Unauthorized("refresh session invalid".into()));
    }
    Ok((selector.to_string(), validator.to_string()))
}

fn random_value(bytes: usize) -> String {
    let mut value = vec![0_u8; bytes];
    OsRng.fill_bytes(&mut value);
    URL_SAFE_NO_PAD.encode(value)
}

fn hash_value(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn unix_timestamp() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize
}

/// Middleware Axum untuk memvalidasi JWT Bearer token pada endpoint MCP HTTP.
pub async fn mcp_auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer(request.headers())?;
    decode_access_token(&state, &token)?;
    Ok(next.run(request).await)
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
        .filter(|(idx, _)| idx % 2 == 1)
        .map(|(_, part)| part)
        .filter_map(|part| part.trim().parse::<i64>().ok())
        .collect()
}

#[derive(Debug, FromRow)]
struct GroupAccessRow {
    module_id: i64,
    module_name: String,
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
        "SELECT ga.module_id, m.module_name, MAX(ga.r) AS r, MAX(ga.w) AS w \
         FROM group_access ga JOIN mst_module m ON m.module_id = ga.module_id \
         WHERE ga.group_id IN (",
    );

    let mut separated = builder.separated(",");
    for group_id in group_ids {
        separated.push_bind(group_id);
    }
    builder.push(") GROUP BY ga.module_id, m.module_name");

    let rows = builder
        .build_query_as::<GroupAccessRow>()
        .fetch_all(&state.pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| ModulePermission {
            module_id: row.module_id,
            module_name: row.module_name,
            read: row.r != 0,
            write: row.w != 0,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::mysql::MySqlPoolOptions;

    fn app_state() -> AppState {
        AppState {
            pool: MySqlPoolOptions::new()
                .connect_lazy("mysql://root:password@127.0.0.1/test")
                .expect("lazy pool"),
            jwt_secret: extract_secret("test-secret-that-is-at-least-32-bytes".into()),
            jwt_issuer: extract_secret("slims-api".into()),
            jwt_audience: extract_secret("slims-admin".into()),
            access_token_ttl: std::time::Duration::from_secs(600),
            refresh_session_ttl: std::time::Duration::from_secs(43_200),
            refresh_remember_ttl: std::time::Duration::from_secs(2_592_000),
            cookie_secure: true,
        }
    }

    fn auth_user(access: Vec<ModulePermission>) -> AuthUser {
        AuthUser {
            claims: Claims {
                sub: "1".into(),
                username: "tester".into(),
                role: Role::Staff,
                access,
                exp: usize::MAX,
                iat: 1,
                nbf: 1,
                iss: "slims-api".into(),
                aud: "slims-admin".into(),
                jti: "test-token".into(),
            },
        }
    }

    #[test]
    fn permissions_are_module_aware() {
        let user = auth_user(vec![ModulePermission {
            module_id: 42,
            module_name: ModuleAccess::Bibliography.name().into(),
            read: true,
            write: false,
        }]);

        assert!(user.can_access(ModuleAccess::Bibliography, Permission::Read));
        assert!(!user.can_access(ModuleAccess::Bibliography, Permission::Write));
        assert!(!user.can_access(ModuleAccess::Membership, Permission::Read));
    }

    #[test]
    fn write_permission_implies_read() {
        let user = auth_user(vec![ModulePermission {
            module_id: 99,
            module_name: ModuleAccess::Circulation.name().into(),
            read: false,
            write: true,
        }]);
        assert!(user.can_access(ModuleAccess::Circulation, Permission::Read));
    }

    #[test]
    fn remember_me_defaults_to_false() {
        let request: LoginRequest = serde_json::from_value(serde_json::json!({
            "username": "tester",
            "password": "secret"
        }))
        .expect("login request should deserialize");

        assert!(!request.remember_me);
    }

    #[test]
    fn refresh_token_parser_rejects_malformed_values() {
        assert!(parse_refresh_token("missing-separator").is_err());
        assert!(parse_refresh_token("short.value").is_err());
    }

    #[test]
    fn generated_refresh_token_has_expected_entropy_and_shape() {
        let selector = random_value(16);
        let validator = random_value(32);
        let parsed = parse_refresh_token(&format!("{selector}.{validator}"))
            .expect("generated token should parse");

        assert_eq!(parsed.0.len(), 22);
        assert_eq!(parsed.1.len(), 43);
        assert_ne!(hash_value(&parsed.1), parsed.1);
    }

    #[tokio::test]
    async fn refresh_cookie_matches_transport_security_mode() {
        let token = IssuedRefreshToken {
            raw: format!("{}.{}", random_value(16), random_value(32)),
            expires_at: Utc::now().naive_utc() + ChronoDuration::hours(1),
            remember_me: false,
        };
        let secure_state = app_state();
        let secure_cookie = build_refresh_cookie(&secure_state, &token);
        assert!(secure_cookie.starts_with("__Host-slims-refresh="));
        assert!(secure_cookie.contains("; Secure"));

        let mut local_http_state = secure_state;
        local_http_state.cookie_secure = false;
        let local_cookie = build_refresh_cookie(&local_http_state, &token);
        assert!(local_cookie.starts_with("slims_refresh="));
        assert!(!local_cookie.contains("; Secure"));
    }

    #[test]
    fn dummy_password_hash_keeps_unknown_user_verification_realistic() {
        assert!(verify("invalid-password", DUMMY_PASSWORD_HASH).unwrap());
        assert!(!verify("different-password", DUMMY_PASSWORD_HASH).unwrap());
    }

    #[tokio::test]
    async fn access_token_requires_expected_issuer_and_audience() {
        let state = app_state();
        let user = User {
            user_id: 7,
            username: "tester".into(),
            passwd: String::new(),
            groups: None,
            user_type: None,
        };
        let issued = issue_access_token(&state, &user, Role::Staff, Vec::new())
            .expect("token should be issued");
        assert!(decode_access_token(&state, &issued.response.token).is_ok());

        let mut wrong_audience = state.clone();
        wrong_audience.jwt_audience = extract_secret("another-client".into());
        assert!(decode_access_token(&wrong_audience, &issued.response.token).is_err());
    }
}
