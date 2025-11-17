use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;

use crate::{
    auth::{AuthUser, Role},
    config::AppState,
    error::AppError,
    resources::{ListParams, PagedResponse},
};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Item {
    pub item_id: i64,
    pub item_code: Option<String>,
    pub biblio_id: Option<i32>,
    pub call_number: Option<String>,
    pub coll_type_id: Option<i32>,
    pub location_id: Option<String>,
    pub item_status_id: Option<String>,
    pub last_update: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct CreateItem {
    pub item_code: Option<String>,
    pub biblio_id: Option<i32>,
    pub call_number: Option<String>,
    pub coll_type_id: Option<i32>,
    pub location_id: Option<String>,
    pub item_status_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct BiblioSummary {
    pub biblio_id: i64,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct CollTypeSummary {
    pub coll_type_id: i64,
    pub coll_type_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct LocationSummary {
    pub location_id: String,
    pub location_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct ItemStatusSummary {
    pub item_status_id: String,
    pub item_status_name: String,
    pub no_loan: i16,
}

#[derive(Debug, Serialize)]
pub struct ItemResponse {
    #[serde(flatten)]
    pub item: Item,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biblio: Option<BiblioSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coll_type: Option<CollTypeSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<LocationSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_status: Option<ItemStatusSummary>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_items).post(create_item))
        .route(
            "/:item_id",
            get(get_item).put(update_item).delete(delete_item),
        )
}

async fn list_items(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<PagedResponse<ItemResponse>>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff, Role::Member])?;

    let pagination = params.pagination();
    let includes = params.includes();
    let (limit, offset, page, per_page) = pagination.limit_offset();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item")
        .fetch_one(&state.pool)
        .await?;

    let items = sqlx::query_as::<_, Item>(
        "SELECT item_id, item_code, biblio_id, call_number, coll_type_id, location_id, item_status_id, last_update FROM item ORDER BY item_id DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut biblio_cache: HashMap<i32, BiblioSummary> = HashMap::new();
    let mut coll_type_cache: HashMap<i32, CollTypeSummary> = HashMap::new();
    let mut location_cache: HashMap<String, LocationSummary> = HashMap::new();
    let mut status_cache: HashMap<String, ItemStatusSummary> = HashMap::new();
    let mut data = Vec::with_capacity(items.len());

    for item in items {
        let mut biblio = None;
        if includes.contains("biblio") {
            if let Some(biblio_id) = item.biblio_id {
                if let Some(existing) = biblio_cache.get(&biblio_id) {
                    biblio = Some(existing.clone());
                } else if let Some(row) = sqlx::query_as::<_, BiblioSummary>(
                    "SELECT biblio_id, title FROM biblio WHERE biblio_id = ?",
                )
                .bind(biblio_id)
                .fetch_optional(&state.pool)
                .await?
                {
                    biblio_cache.insert(biblio_id, row.clone());
                    biblio = Some(row);
                }
            }
        }

        let mut coll_type = None;
        if includes.contains("coll_type") {
            if let Some(coll_type_id) = item.coll_type_id {
                if let Some(existing) = coll_type_cache.get(&coll_type_id) {
                    coll_type = Some(existing.clone());
                } else if let Some(row) = sqlx::query_as::<_, CollTypeSummary>(
                    "SELECT coll_type_id, coll_type_name FROM mst_coll_type WHERE coll_type_id = ?",
                )
                .bind(coll_type_id)
                .fetch_optional(&state.pool)
                .await?
                {
                    coll_type_cache.insert(coll_type_id, row.clone());
                    coll_type = Some(row);
                }
            }
        }

        let mut location = None;
        if includes.contains("location") {
            if let Some(loc_id) = item.location_id.clone() {
                if let Some(existing) = location_cache.get(&loc_id) {
                    location = Some(existing.clone());
                } else if let Some(row) = sqlx::query_as::<_, LocationSummary>(
                    "SELECT location_id, location_name FROM mst_location WHERE location_id = ?",
                )
                .bind(&loc_id)
                .fetch_optional(&state.pool)
                .await?
                {
                    location_cache.insert(loc_id.clone(), row.clone());
                    location = Some(row);
                }
            }
        }

        let mut item_status = None;
        if includes.contains("item_status") {
            if let Some(status_id) = item.item_status_id.clone() {
                if let Some(existing) = status_cache.get(&status_id) {
                    item_status = Some(existing.clone());
                } else if let Some(row) = sqlx::query_as::<_, ItemStatusSummary>(
                    "SELECT item_status_id, item_status_name, no_loan FROM mst_item_status WHERE item_status_id = ?",
                )
                .bind(&status_id)
                .fetch_optional(&state.pool)
                .await?
                {
                    status_cache.insert(status_id.clone(), row.clone());
                    item_status = Some(row);
                }
            }
        }

        data.push(ItemResponse {
            item,
            biblio,
            coll_type,
            location,
            item_status,
        });
    }

    Ok(Json(PagedResponse {
        data,
        page,
        per_page,
        total,
    }))
}

async fn get_item(
    State(state): State<AppState>,
    Path(item_id): Path<i64>,
    auth: AuthUser,
) -> Result<Json<Item>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff, Role::Member])?;

    let item = sqlx::query_as::<_, Item>(
        "SELECT item_id, item_code, biblio_id, call_number, coll_type_id, location_id, item_status_id, last_update FROM item WHERE item_id = ?",
    )
    .bind(item_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(item))
}

async fn create_item(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateItem>,
) -> Result<Json<Item>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian])?;

    let now = chrono::Utc::now().naive_utc();

    let result = sqlx::query(
        "INSERT INTO item (item_code, biblio_id, call_number, coll_type_id, location_id, item_status_id, input_date) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&payload.item_code)
    .bind(payload.biblio_id)
    .bind(&payload.call_number)
    .bind(payload.coll_type_id)
    .bind(&payload.location_id)
    .bind(&payload.item_status_id)
    .bind(now)
    .execute(&state.pool)
    .await?;

    let rec = sqlx::query_as::<_, Item>(
        "SELECT item_id, item_code, biblio_id, call_number, coll_type_id, location_id, item_status_id, last_update FROM item WHERE item_id = ?",
    )
    .bind(result.last_insert_id() as i64)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(rec))
}

async fn update_item(
    State(state): State<AppState>,
    Path(item_id): Path<i64>,
    auth: AuthUser,
    Json(payload): Json<CreateItem>,
) -> Result<Json<Item>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian])?;

    let updated = sqlx::query(
        "UPDATE item SET item_code = ?, biblio_id = ?, call_number = ?, coll_type_id = ?, location_id = ?, item_status_id = ?, last_update = NOW() WHERE item_id = ?",
    )
    .bind(&payload.item_code)
    .bind(payload.biblio_id)
    .bind(&payload.call_number)
    .bind(payload.coll_type_id)
    .bind(&payload.location_id)
    .bind(&payload.item_status_id)
    .bind(item_id)
    .execute(&state.pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let rec = sqlx::query_as::<_, Item>(
        "SELECT item_id, item_code, biblio_id, call_number, coll_type_id, location_id, item_status_id, last_update FROM item WHERE item_id = ?",
    )
    .bind(item_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(rec))
}

async fn delete_item(
    State(state): State<AppState>,
    Path(item_id): Path<i64>,
    auth: AuthUser,
) -> Result<StatusCode, AppError> {
    auth.require_roles(&[Role::Admin])?;

    sqlx::query("DELETE FROM item WHERE item_id = ?")
        .bind(item_id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
