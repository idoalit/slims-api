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
pub struct Biblio {
    pub biblio_id: i64,
    pub title: String,
    pub gmd_id: Option<i32>,
    pub publisher_id: Option<i32>,
    pub publish_year: Option<String>,
    pub language_id: Option<String>,
    pub classification: Option<String>,
    pub call_number: Option<String>,
    pub opac_hide: Option<i16>,
    pub promoted: Option<i16>,
    pub input_date: Option<NaiveDateTime>,
    pub last_update: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertBiblio {
    pub title: String,
    pub gmd_id: Option<i32>,
    pub publisher_id: Option<i32>,
    pub publish_year: Option<String>,
    pub language_id: Option<String>,
    pub classification: Option<String>,
    pub call_number: Option<String>,
    pub opac_hide: Option<i16>,
    pub promoted: Option<i16>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct GmdInfo {
    pub gmd_id: i64,
    pub gmd_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct PublisherInfo {
    pub publisher_id: i64,
    pub publisher_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct LanguageInfo {
    pub language_id: String,
    pub language_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct AuthorInfo {
    pub author_id: i64,
    pub author_name: String,
    pub authority_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct TopicInfo {
    pub topic_id: i64,
    pub topic: String,
    pub topic_type: String,
}

#[derive(Debug, Serialize)]
pub struct BiblioResponse {
    #[serde(flatten)]
    pub biblio: Biblio,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gmd: Option<GmdInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<PublisherInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<AuthorInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<TopicInfo>>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_biblios).post(create_biblio))
        .route(
            "/:biblio_id",
            get(get_biblio).put(update_biblio).delete(delete_biblio),
        )
}

async fn list_biblios(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<PagedResponse<BiblioResponse>>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff])?;

    let pagination = params.pagination();
    let includes = params.includes();
    let (limit, offset, page, per_page) = pagination.limit_offset();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM biblio")
        .fetch_one(&state.pool)
        .await?;

    let rows = sqlx::query_as::<_, Biblio>(
        "SELECT biblio_id, title, gmd_id, publisher_id, publish_year, language_id, classification, call_number, opac_hide, promoted, input_date, last_update FROM biblio ORDER BY biblio_id DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut gmd_cache: HashMap<i32, GmdInfo> = HashMap::new();
    let mut publisher_cache: HashMap<i32, PublisherInfo> = HashMap::new();
    let mut language_cache: HashMap<String, LanguageInfo> = HashMap::new();
    let mut data = Vec::with_capacity(rows.len());

    for biblio in rows {
        let mut gmd = None;
        if includes.contains("gmd") {
            if let Some(gmd_id) = biblio.gmd_id {
                if let Some(existing) = gmd_cache.get(&gmd_id) {
                    gmd = Some(existing.clone());
                } else if let Some(row) = sqlx::query_as::<_, GmdInfo>(
                    "SELECT gmd_id, gmd_name FROM mst_gmd WHERE gmd_id = ?",
                )
                .bind(gmd_id)
                .fetch_optional(&state.pool)
                .await?
                {
                    gmd_cache.insert(gmd_id, row.clone());
                    gmd = Some(row);
                }
            }
        }

        let mut publisher = None;
        if includes.contains("publisher") {
            if let Some(pub_id) = biblio.publisher_id {
                if let Some(existing) = publisher_cache.get(&pub_id) {
                    publisher = Some(existing.clone());
                } else if let Some(row) = sqlx::query_as::<_, PublisherInfo>(
                    "SELECT publisher_id, publisher_name FROM mst_publisher WHERE publisher_id = ?",
                )
                .bind(pub_id)
                .fetch_optional(&state.pool)
                .await?
                {
                    publisher_cache.insert(pub_id, row.clone());
                    publisher = Some(row);
                }
            }
        }

        let mut language = None;
        if includes.contains("language") {
            if let Some(lang_id) = biblio.language_id.clone() {
                if let Some(existing) = language_cache.get(&lang_id) {
                    language = Some(existing.clone());
                } else if let Some(row) = sqlx::query_as::<_, LanguageInfo>(
                    "SELECT language_id, language_name FROM mst_language WHERE language_id = ?",
                )
                .bind(&lang_id)
                .fetch_optional(&state.pool)
                .await?
                {
                    language_cache.insert(lang_id.clone(), row.clone());
                    language = Some(row);
                }
            }
        }

        let authors = if includes.contains("authors") {
            let rows = sqlx::query_as::<_, AuthorInfo>(
                "SELECT a.author_id, a.author_name, a.authority_type FROM biblio_author ba JOIN mst_author a ON ba.author_id = a.author_id WHERE ba.biblio_id = ?",
            )
            .bind(biblio.biblio_id)
            .fetch_all(&state.pool)
            .await?;
            Some(rows)
        } else {
            None
        };

        let topics = if includes.contains("topics") {
            let rows = sqlx::query_as::<_, TopicInfo>(
                "SELECT t.topic_id, t.topic, t.topic_type FROM biblio_topic bt JOIN mst_topic t ON bt.topic_id = t.topic_id WHERE bt.biblio_id = ?",
            )
            .bind(biblio.biblio_id)
            .fetch_all(&state.pool)
            .await?;
            Some(rows)
        } else {
            None
        };

        data.push(BiblioResponse {
            biblio,
            gmd,
            publisher,
            language,
            authors,
            topics,
        });
    }

    Ok(Json(PagedResponse {
        data,
        page,
        per_page,
        total,
    }))
}

async fn get_biblio(
    State(state): State<AppState>,
    Path(biblio_id): Path<i64>,
    auth: AuthUser,
) -> Result<Json<Biblio>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff, Role::Member])?;

    let row = sqlx::query_as::<_, Biblio>(
        "SELECT biblio_id, title, gmd_id, publisher_id, publish_year, language_id, classification, call_number, opac_hide, promoted, input_date, last_update FROM biblio WHERE biblio_id = ?",
    )
    .bind(biblio_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

async fn create_biblio(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpsertBiblio>,
) -> Result<Json<Biblio>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian])?;

    let now = chrono::Utc::now().naive_utc();

    let result = sqlx::query(
        "INSERT INTO biblio (title, gmd_id, publisher_id, publish_year, language_id, classification, call_number, opac_hide, promoted, input_date, last_update) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&payload.title)
    .bind(payload.gmd_id)
    .bind(payload.publisher_id)
    .bind(&payload.publish_year)
    .bind(&payload.language_id)
    .bind(&payload.classification)
    .bind(&payload.call_number)
    .bind(payload.opac_hide.unwrap_or(0))
    .bind(payload.promoted.unwrap_or(0))
    .bind(now)
    .bind(now)
    .execute(&state.pool)
    .await?;

    let rec = sqlx::query_as::<_, Biblio>("SELECT biblio_id, title, gmd_id, publisher_id, publish_year, language_id, classification, call_number, opac_hide, promoted, input_date, last_update FROM biblio WHERE biblio_id = ?")
        .bind(result.last_insert_id() as i64)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(rec))
}

async fn update_biblio(
    State(state): State<AppState>,
    Path(biblio_id): Path<i64>,
    auth: AuthUser,
    Json(payload): Json<UpsertBiblio>,
) -> Result<Json<Biblio>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian])?;

    let now = chrono::Utc::now().naive_utc();

    let updated = sqlx::query(
        "UPDATE biblio SET title = ?, gmd_id = ?, publisher_id = ?, publish_year = ?, language_id = ?, classification = ?, call_number = ?, opac_hide = ?, promoted = ?, last_update = ? WHERE biblio_id = ?",
    )
    .bind(&payload.title)
    .bind(payload.gmd_id)
    .bind(payload.publisher_id)
    .bind(&payload.publish_year)
    .bind(&payload.language_id)
    .bind(&payload.classification)
    .bind(&payload.call_number)
    .bind(payload.opac_hide.unwrap_or(0))
    .bind(payload.promoted.unwrap_or(0))
    .bind(now)
    .bind(biblio_id)
    .execute(&state.pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let rec = sqlx::query_as::<_, Biblio>("SELECT biblio_id, title, gmd_id, publisher_id, publish_year, language_id, classification, call_number, opac_hide, promoted, input_date, last_update FROM biblio WHERE biblio_id = ?")
        .bind(biblio_id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(rec))
}

async fn delete_biblio(
    State(state): State<AppState>,
    Path(biblio_id): Path<i64>,
    auth: AuthUser,
) -> Result<StatusCode, AppError> {
    auth.require_roles(&[Role::Admin])?;

    sqlx::query("DELETE FROM biblio WHERE biblio_id = ?")
        .bind(biblio_id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
