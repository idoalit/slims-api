use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::mysql::MySqlRow;
use sqlx::{Column, FromRow, Row};
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::{
    auth::{AuthUser, ModuleAccess, Permission},
    config::AppState,
    error::AppError,
    resources::{ListParams, PagedResponse},
};

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Member {
    pub member_id: String,
    pub member_name: String,
    pub member_email: Option<String>,
    pub member_type_id: Option<i32>,
    pub expire_date: NaiveDate,
    pub is_pending: i16,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMember {
    pub member_id: String,
    pub member_name: String,
    pub member_email: Option<String>,
    pub member_type_id: Option<i32>,
    pub expire_date: NaiveDate,
    pub gender: Option<i16>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct MemberTypeInfo {
    pub member_type_id: i64,
    pub member_type_name: String,
    pub loan_limit: i64,
    pub loan_periode: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberResponse {
    #[serde(flatten)]
    pub member: Member,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_type: Option<MemberTypeInfo>,
    #[schema(value_type = Object)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<JsonValue>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_members).post(create_member))
        .route(
            "/:member_id",
            get(get_member).put(update_member).delete(delete_member),
        )
}

#[utoipa::path(
    get,
    path = "/members",
    responses((status = 200, description = "Paginated members", body = PagedMembers)),
    security(("bearerAuth" = [])),
    tag = "Members"
)]
async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<PagedResponse<MemberResponse>>, AppError> {
    auth.require_access(ModuleAccess::Membership, Permission::Read)?;

    let pagination = params.pagination();
    let includes = params.includes();
    let (limit, offset, page, per_page) = pagination.limit_offset();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM member")
        .fetch_one(&state.pool)
        .await?;

    let members = sqlx::query_as::<_, Member>(
        "SELECT member_id, member_name, member_email, member_type_id, expire_date, is_pending FROM member ORDER BY register_date DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut member_type_cache: HashMap<i32, MemberTypeInfo> = HashMap::new();
    let mut data = Vec::with_capacity(members.len());

    for member in members {
        let mut member_type = None;
        if includes.contains("member_type") {
            if let Some(mt_id) = member.member_type_id {
                if let Some(existing) = member_type_cache.get(&mt_id) {
                    member_type = Some(existing.clone());
                } else {
                    if let Some(mt) = sqlx::query_as::<_, MemberTypeInfo>(
                        "SELECT member_type_id, member_type_name, loan_limit, loan_periode FROM mst_member_type WHERE member_type_id = ?",
                    )
                    .bind(mt_id)
                    .fetch_optional(&state.pool)
                    .await?
                    {
                        member_type_cache.insert(mt_id, mt.clone());
                        member_type = Some(mt);
                    }
                }
            }
        }

        let custom = if includes.contains("custom") {
            if let Some(row) = sqlx::query("SELECT * FROM member_custom WHERE member_id = ?")
                .bind(&member.member_id)
                .fetch_optional(&state.pool)
                .await?
            {
                Some(row_to_json(&row))
            } else {
                None
            }
        } else {
            None
        };

        data.push(MemberResponse {
            member,
            member_type,
            custom,
        });
    }

    Ok(Json(PagedResponse {
        data,
        page,
        per_page,
        total,
    }))
}

#[utoipa::path(
    get,
    path = "/members/{member_id}",
    params(("member_id" = String, Path, description = "Member ID")),
    responses((status = 200, body = MemberResponse)),
    security(("bearerAuth" = [])),
    tag = "Members"
)]
async fn get_member(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
    Path(member_id): Path<String>,
    auth: AuthUser,
) -> Result<Json<MemberResponse>, AppError> {
    auth.require_access(ModuleAccess::Membership, Permission::Read)?;

    let member = sqlx::query_as::<_, Member>(
        "SELECT member_id, member_name, member_email, member_type_id, expire_date, is_pending FROM member WHERE member_id = ?",
    )
    .bind(&member_id)
    .fetch_one(&state.pool)
    .await?;

    let includes = params.includes();
    let mut member_type = None;
    if includes.contains("member_type") {
        if let Some(mt_id) = member.member_type_id {
            member_type = sqlx::query_as::<_, MemberTypeInfo>(
                "SELECT member_type_id, member_type_name, loan_limit, loan_periode FROM mst_member_type WHERE member_type_id = ?",
            )
            .bind(mt_id)
            .fetch_optional(&state.pool)
            .await?;
        }
    }

    let custom = if includes.contains("custom") {
        if let Some(row) = sqlx::query("SELECT * FROM member_custom WHERE member_id = ?")
            .bind(&member.member_id)
            .fetch_optional(&state.pool)
            .await?
        {
            Some(row_to_json(&row))
        } else {
            None
        }
    } else {
        None
    };

    Ok(Json(MemberResponse {
        member,
        member_type,
        custom,
    }))
}

fn row_to_json(row: &MySqlRow) -> JsonValue {
    let mut map = serde_json::Map::new();
    for (idx, col) in row.columns().iter().enumerate() {
        let key = col.name().to_string();
        let val: Option<String> = row.try_get(idx).ok();
        map.insert(key, val.map(JsonValue::String).unwrap_or(JsonValue::Null));
    }
    JsonValue::Object(map)
}

#[utoipa::path(
    post,
    path = "/members",
    request_body = CreateMember,
    responses((status = 200, body = Member)),
    security(("bearerAuth" = [])),
    tag = "Members"
)]
async fn create_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateMember>,
) -> Result<Json<Member>, AppError> {
    auth.require_access(ModuleAccess::Membership, Permission::Write)?;

    let gender = payload.gender.unwrap_or(0);

    sqlx::query(
        "INSERT INTO member (member_id, member_name, gender, member_email, member_type_id, expire_date, register_date, member_since_date, is_pending) VALUES (?, ?, ?, ?, ?, ?, CURDATE(), CURDATE(), 0)",
    )
    .bind(&payload.member_id)
    .bind(&payload.member_name)
    .bind(gender)
    .bind(&payload.member_email)
    .bind(payload.member_type_id)
    .bind(payload.expire_date)
    .execute(&state.pool)
    .await?;

    let rec = sqlx::query_as::<_, Member>(
        "SELECT member_id, member_name, member_email, member_type_id, expire_date, is_pending FROM member WHERE member_id = ?",
    )
    .bind(&payload.member_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(rec))
}

#[utoipa::path(
    put,
    path = "/members/{member_id}",
    request_body = CreateMember,
    params(("member_id" = String, Path, description = "Member ID")),
    responses((status = 200, body = Member)),
    security(("bearerAuth" = [])),
    tag = "Members"
)]
async fn update_member(
    State(state): State<AppState>,
    Path(member_id): Path<String>,
    auth: AuthUser,
    Json(payload): Json<CreateMember>,
) -> Result<Json<Member>, AppError> {
    auth.require_access(ModuleAccess::Membership, Permission::Write)?;

    let gender = payload.gender.unwrap_or(0);

    let updated = sqlx::query(
        "UPDATE member SET member_id = ?, member_name = ?, gender = ?, member_email = ?, member_type_id = ?, expire_date = ?, last_update = CURDATE() WHERE member_id = ?",
    )
    .bind(&payload.member_id)
    .bind(&payload.member_name)
    .bind(gender)
    .bind(&payload.member_email)
    .bind(payload.member_type_id)
    .bind(payload.expire_date)
    .bind(&member_id)
    .execute(&state.pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let rec = sqlx::query_as::<_, Member>(
        "SELECT member_id, member_name, member_email, member_type_id, expire_date, is_pending FROM member WHERE member_id = ?",
    )
    .bind(&payload.member_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(rec))
}

#[utoipa::path(
    delete,
    path = "/members/{member_id}",
    params(("member_id" = String, Path, description = "Member ID")),
    responses((status = 204, description = "Member deleted")),
    security(("bearerAuth" = [])),
    tag = "Members"
)]
async fn delete_member(
    State(state): State<AppState>,
    Path(member_id): Path<String>,
    auth: AuthUser,
) -> Result<StatusCode, AppError> {
    auth.require_access(ModuleAccess::Membership, Permission::Write)?;

    sqlx::query("DELETE FROM member WHERE member_id = ?")
        .bind(&member_id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
