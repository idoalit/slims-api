use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    auth::{AuthUser, Role},
    config::AppState,
    error::AppError,
    resources::{ListParams, PagedResponse},
};

#[derive(Debug, Serialize, Deserialize, FromRow)]
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

async fn list_contents(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<PagedResponse<Content>>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff])?;

    let pagination = params.pagination();
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

    Ok(Json(PagedResponse {
        data: rows,
        page,
        per_page,
        total,
    }))
}

async fn get_content(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(content_id): Path<i64>,
) -> Result<Json<Content>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff])?;

    let row = sqlx::query_as::<_, Content>(
        "SELECT content_id, content_title, content_desc, content_path, is_news, input_date, last_update, content_ownpage FROM content WHERE content_id = ?",
    )
    .bind(content_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

async fn get_content_by_path(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(content_path): Path<String>,
) -> Result<Json<Content>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff])?;

    let row = sqlx::query_as::<_, Content>(
        "SELECT content_id, content_title, content_desc, content_path, is_news, input_date, last_update, content_ownpage FROM content WHERE content_path = ?",
    )
    .bind(content_path)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}
