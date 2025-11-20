use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::{
    auth::{AuthUser, ModuleAccess, Permission},
    config::AppState,
    error::AppError,
    jsonapi::{
        JsonApiDocument, collection_document, pagination_meta, resource_with_fields,
        single_document,
    },
    resources::ListParams,
};

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Content {
    pub content_id: i64,
    pub content_title: String,
    pub content_desc: String,
    pub content_path: String,
    pub is_news: Option<i16>,
    pub input_date: NaiveDateTime,
    pub last_update: NaiveDateTime,
    pub content_ownpage: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_contents))
        .route("/:content_id", get(get_content))
        .route("/path/:content_path", get(get_content_by_path))
}

#[utoipa::path(
    get,
    path = "/contents",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Contents"
)]
async fn list_contents(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::System, Permission::Read)?;

    let pagination = params.pagination();
    let content_fields = params.fieldset("contents");
    let (limit, offset, page, per_page) = pagination.limit_offset();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM content")
        .fetch_one(&state.pool)
        .await?;

    let rows = sqlx::query_as::<_, Content>(
        "SELECT content_id, content_title, content_desc, content_path, is_news, input_date, last_update, content_ownpage FROM content ORDER BY content_id DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let data = rows
        .into_iter()
        .map(|content| {
            resource_with_fields(
                "contents",
                content.content_id.to_string(),
                content,
                content_fields,
            )
        })
        .collect();

    Ok(Json(collection_document(
        data,
        pagination_meta(page, per_page, total),
    )))
}

#[utoipa::path(
    get,
    path = "/contents/{content_id}",
    params(("content_id" = i64, Path, description = "Content ID")),
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Contents"
)]
async fn get_content(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
    auth: AuthUser,
    Path(content_id): Path<i64>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::System, Permission::Read)?;

    let row = sqlx::query_as::<_, Content>(
        "SELECT content_id, content_title, content_desc, content_path, is_news, input_date, last_update, content_ownpage FROM content WHERE content_id = ?",
    )
    .bind(content_id)
    .fetch_one(&state.pool)
    .await?;

    let content_fields = params.fieldset("contents");
    Ok(Json(single_document(resource_with_fields(
        "contents",
        row.content_id.to_string(),
        row,
        content_fields,
    ))))
}

#[utoipa::path(
    get,
    path = "/contents/path/{content_path}",
    params(("content_path" = String, Path, description = "Content path slug")),
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Contents"
)]
async fn get_content_by_path(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
    auth: AuthUser,
    Path(content_path): Path<String>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::System, Permission::Read)?;

    let row = sqlx::query_as::<_, Content>(
        "SELECT content_id, content_title, content_desc, content_path, is_news, input_date, last_update, content_ownpage FROM content WHERE content_path = ?",
    )
    .bind(content_path)
    .fetch_one(&state.pool)
    .await?;

    let content_fields = params.fieldset("contents");
    Ok(Json(single_document(resource_with_fields(
        "contents",
        row.content_id.to_string(),
        row,
        content_fields,
    ))))
}
