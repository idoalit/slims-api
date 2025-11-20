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
pub struct Visitor {
    pub visitor_id: i64,
    pub member_id: Option<String>,
    pub member_name: String,
    pub institution: Option<String>,
    pub checkin_date: NaiveDateTime,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_visitors))
        .route("/:visitor_id", get(get_visitor))
}

#[utoipa::path(
    get,
    path = "/visitors",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Visitors"
)]
async fn list_visitors(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Membership, Permission::Read)?;

    let pagination = params.pagination();
    let visitor_fields = params.fieldset("visitors");
    let (limit, offset, page, per_page) = pagination.limit_offset();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM visitor_count")
        .fetch_one(&state.pool)
        .await?;

    let rows = sqlx::query_as::<_, Visitor>(
        "SELECT visitor_id, member_id, member_name, institution, checkin_date FROM visitor_count ORDER BY checkin_date DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let data = rows
        .into_iter()
        .map(|visitor| {
            resource_with_fields(
                "visitors",
                visitor.visitor_id.to_string(),
                visitor,
                visitor_fields,
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
    path = "/visitors/{visitor_id}",
    params(("visitor_id" = i64, Path, description = "Visitor ID")),
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Visitors"
)]
async fn get_visitor(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
    auth: AuthUser,
    Path(visitor_id): Path<i64>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Membership, Permission::Read)?;

    let row = sqlx::query_as::<_, Visitor>(
        "SELECT visitor_id, member_id, member_name, institution, checkin_date FROM visitor_count WHERE visitor_id = ?",
    )
    .bind(visitor_id)
    .fetch_one(&state.pool)
    .await?;

    let visitor_fields = params.fieldset("visitors");
    Ok(Json(single_document(resource_with_fields(
        "visitors",
        row.visitor_id.to_string(),
        row,
        visitor_fields,
    ))))
}
