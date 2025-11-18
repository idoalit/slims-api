use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, Row, Column};
use std::collections::HashMap;
use sqlx::mysql::MySqlRow;

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
    pub content_type_id: Option<i32>,
    pub media_type_id: Option<i32>,
    pub carrier_type_id: Option<i32>,
    pub frequency_id: Option<i32>,
    pub publish_place_id: Option<i32>,
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
pub struct ContentTypeInfo {
    pub id: i64,
    pub content_type: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct MediaTypeInfo {
    pub id: i64,
    pub media_type: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct CarrierTypeInfo {
    pub id: i64,
    pub carrier_type: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct FrequencyInfo {
    pub frequency_id: i64,
    pub frequency: String,
    pub language_prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct PlaceInfo {
    pub place_id: i64,
    pub place_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct ItemSummary {
    pub item_id: i64,
    pub item_code: Option<String>,
    pub call_number: Option<String>,
    pub coll_type_id: Option<i32>,
    pub location_id: Option<String>,
    pub item_status_id: Option<String>,
    pub last_update: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct AttachmentInfo {
    pub file_id: i64,
    pub file_title: String,
    pub file_name: String,
    pub file_url: Option<String>,
    pub file_dir: Option<String>,
    pub mime_type: Option<String>,
    pub placement: Option<String>,
    pub access_type: String,
    pub access_limit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct BiblioRelationInfo {
    pub biblio_id: i64,
    pub title: String,
    pub rel_type: i32,
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
    pub content_type: Option<ContentTypeInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<MediaTypeInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carrier_type: Option<CarrierTypeInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<FrequencyInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<PlaceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<AuthorInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<TopicInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ItemSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relations: Option<Vec<BiblioRelationInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<AttachmentInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<JsonValue>,
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
        "SELECT biblio_id, title, gmd_id, publisher_id, publish_year, language_id, content_type_id, media_type_id, carrier_type_id, frequency_id, publish_place_id, classification, call_number, opac_hide, promoted, input_date, last_update FROM biblio ORDER BY biblio_id DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut gmd_cache: HashMap<i32, GmdInfo> = HashMap::new();
    let mut publisher_cache: HashMap<i32, PublisherInfo> = HashMap::new();
    let mut language_cache: HashMap<String, LanguageInfo> = HashMap::new();
    let mut content_type_cache: HashMap<i32, ContentTypeInfo> = HashMap::new();
    let mut media_type_cache: HashMap<i32, MediaTypeInfo> = HashMap::new();
    let mut carrier_type_cache: HashMap<i32, CarrierTypeInfo> = HashMap::new();
    let mut frequency_cache: HashMap<i32, FrequencyInfo> = HashMap::new();
    let mut place_cache: HashMap<i32, PlaceInfo> = HashMap::new();
    let mut data = Vec::with_capacity(rows.len());

    for biblio in rows {
        let custom = if includes.contains("custom") {
            if let Some(row) = sqlx::query("SELECT * FROM biblio_custom WHERE biblio_id = ?")
                .bind(biblio.biblio_id)
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

        let mut content_type = None;
        if includes.contains("content_type") {
            if let Some(ct_id) = biblio.content_type_id {
                if ct_id > 0 {
                    if let Some(existing) = content_type_cache.get(&ct_id) {
                        content_type = Some(existing.clone());
                    } else if let Some(row) = sqlx::query_as::<_, ContentTypeInfo>(
                        "SELECT id, content_type, code FROM mst_content_type WHERE id = ?",
                    )
                    .bind(ct_id)
                    .fetch_optional(&state.pool)
                    .await?
                    {
                        content_type_cache.insert(ct_id, row.clone());
                        content_type = Some(row);
                    }
                }
            }
        }

        let mut media_type = None;
        if includes.contains("media_type") {
            if let Some(mt_id) = biblio.media_type_id {
                if mt_id > 0 {
                    if let Some(existing) = media_type_cache.get(&mt_id) {
                        media_type = Some(existing.clone());
                    } else if let Some(row) = sqlx::query_as::<_, MediaTypeInfo>(
                        "SELECT id, media_type, code FROM mst_media_type WHERE id = ?",
                    )
                    .bind(mt_id)
                    .fetch_optional(&state.pool)
                    .await?
                    {
                        media_type_cache.insert(mt_id, row.clone());
                        media_type = Some(row);
                    }
                }
            }
        }

        let mut carrier_type = None;
        if includes.contains("carrier_type") {
            if let Some(ct_id) = biblio.carrier_type_id {
                if ct_id > 0 {
                    if let Some(existing) = carrier_type_cache.get(&ct_id) {
                        carrier_type = Some(existing.clone());
                    } else if let Some(row) = sqlx::query_as::<_, CarrierTypeInfo>(
                        "SELECT id, carrier_type, code FROM mst_carrier_type WHERE id = ?",
                    )
                    .bind(ct_id)
                    .fetch_optional(&state.pool)
                    .await?
                    {
                        carrier_type_cache.insert(ct_id, row.clone());
                        carrier_type = Some(row);
                    }
                }
            }
        }

        let mut frequency = None;
        if includes.contains("frequency") {
            if let Some(freq_id) = biblio.frequency_id {
                if freq_id > 0 {
                    if let Some(existing) = frequency_cache.get(&freq_id) {
                        frequency = Some(existing.clone());
                    } else if let Some(row) = sqlx::query_as::<_, FrequencyInfo>(
                        "SELECT frequency_id, frequency, language_prefix FROM mst_frequency WHERE frequency_id = ?",
                    )
                    .bind(freq_id)
                    .fetch_optional(&state.pool)
                    .await?
                    {
                        frequency_cache.insert(freq_id, row.clone());
                        frequency = Some(row);
                    }
                }
            }
        }

        let mut place = None;
        if includes.contains("place") {
            if let Some(place_id) = biblio.publish_place_id {
                if place_id > 0 {
                    if let Some(existing) = place_cache.get(&place_id) {
                        place = Some(existing.clone());
                    } else if let Some(row) = sqlx::query_as::<_, PlaceInfo>(
                        "SELECT place_id, place_name FROM mst_place WHERE place_id = ?",
                    )
                    .bind(place_id)
                    .fetch_optional(&state.pool)
                    .await?
                    {
                        place_cache.insert(place_id, row.clone());
                        place = Some(row);
                    }
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

        let items = if includes.contains("items") {
            let rows = sqlx::query_as::<_, ItemSummary>(
                "SELECT item_id, item_code, call_number, coll_type_id, location_id, item_status_id, last_update FROM item WHERE biblio_id = ? ORDER BY item_id DESC",
            )
            .bind(biblio.biblio_id)
            .fetch_all(&state.pool)
            .await?;
            Some(rows)
        } else {
            None
        };

        let attachments = if includes.contains("attachments") || includes.contains("files") {
            let rows = sqlx::query_as::<_, AttachmentInfo>(
                "SELECT f.file_id, f.file_title, f.file_name, f.file_url, f.file_dir, f.mime_type, ba.placement, ba.access_type, ba.access_limit FROM biblio_attachment ba JOIN files f ON f.file_id = ba.file_id WHERE ba.biblio_id = ? ORDER BY ba.file_id DESC",
            )
            .bind(biblio.biblio_id)
            .fetch_all(&state.pool)
            .await?;
            Some(rows)
        } else {
            None
        };

        let relations = if includes.contains("relations") {
            let rows = sqlx::query_as::<_, BiblioRelationInfo>(
                "SELECT br.rel_biblio_id AS biblio_id, b.title, br.rel_type FROM biblio_relation br JOIN biblio b ON b.biblio_id = br.rel_biblio_id WHERE br.biblio_id = ?",
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
            content_type,
            media_type,
            carrier_type,
            frequency,
            place,
            authors,
            topics,
            items,
            relations,
            attachments,
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

async fn get_biblio(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
    Path(biblio_id): Path<i64>,
    auth: AuthUser,
) -> Result<Json<BiblioResponse>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff, Role::Member])?;

    let row = sqlx::query_as::<_, Biblio>(
        "SELECT biblio_id, title, gmd_id, publisher_id, publish_year, language_id, content_type_id, media_type_id, carrier_type_id, frequency_id, publish_place_id, classification, call_number, opac_hide, promoted, input_date, last_update FROM biblio WHERE biblio_id = ?",
    )
    .bind(biblio_id)
    .fetch_one(&state.pool)
    .await?;

    let includes = params.includes();

    let gmd = if includes.contains("gmd") {
        if let Some(gmd_id) = row.gmd_id {
            sqlx::query_as::<_, GmdInfo>("SELECT gmd_id, gmd_name FROM mst_gmd WHERE gmd_id = ?")
                .bind(gmd_id)
                .fetch_optional(&state.pool)
                .await?
        } else {
            None
        }
    } else {
        None
    };

    let publisher = if includes.contains("publisher") {
        if let Some(pub_id) = row.publisher_id {
            sqlx::query_as::<_, PublisherInfo>(
                "SELECT publisher_id, publisher_name FROM mst_publisher WHERE publisher_id = ?",
            )
            .bind(pub_id)
            .fetch_optional(&state.pool)
            .await?
        } else {
            None
        }
    } else {
        None
    };

    let language = if includes.contains("language") {
        if let Some(lang_id) = row.language_id.clone() {
            sqlx::query_as::<_, LanguageInfo>(
                "SELECT language_id, language_name FROM mst_language WHERE language_id = ?",
            )
            .bind(&lang_id)
            .fetch_optional(&state.pool)
            .await?
        } else {
            None
        }
    } else {
        None
    };

    let content_type = if includes.contains("content_type") {
        if let Some(ct_id) = row.content_type_id {
            sqlx::query_as::<_, ContentTypeInfo>(
                "SELECT id, content_type, code FROM mst_content_type WHERE id = ?",
            )
            .bind(ct_id)
            .fetch_optional(&state.pool)
            .await?
        } else {
            None
        }
    } else {
        None
    };

    let media_type = if includes.contains("media_type") {
        if let Some(mt_id) = row.media_type_id {
            sqlx::query_as::<_, MediaTypeInfo>(
                "SELECT id, media_type, code FROM mst_media_type WHERE id = ?",
            )
            .bind(mt_id)
            .fetch_optional(&state.pool)
            .await?
        } else {
            None
        }
    } else {
        None
    };

    let carrier_type = if includes.contains("carrier_type") {
        if let Some(ct_id) = row.carrier_type_id {
            sqlx::query_as::<_, CarrierTypeInfo>(
                "SELECT id, carrier_type, code FROM mst_carrier_type WHERE id = ?",
            )
            .bind(ct_id)
            .fetch_optional(&state.pool)
            .await?
        } else {
            None
        }
    } else {
        None
    };

    let frequency = if includes.contains("frequency") {
        if let Some(freq_id) = row.frequency_id {
            sqlx::query_as::<_, FrequencyInfo>(
                "SELECT frequency_id, frequency, language_prefix FROM mst_frequency WHERE frequency_id = ?",
            )
            .bind(freq_id)
            .fetch_optional(&state.pool)
            .await?
        } else {
            None
        }
    } else {
        None
    };

    let place = if includes.contains("place") {
        if let Some(place_id) = row.publish_place_id {
            sqlx::query_as::<_, PlaceInfo>(
                "SELECT place_id, place_name FROM mst_place WHERE place_id = ?",
            )
            .bind(place_id)
            .fetch_optional(&state.pool)
            .await?
        } else {
            None
        }
    } else {
        None
    };

    let authors = if includes.contains("authors") {
        let rows = sqlx::query_as::<_, AuthorInfo>(
            "SELECT a.author_id, a.author_name, a.authority_type FROM biblio_author ba JOIN mst_author a ON ba.author_id = a.author_id WHERE ba.biblio_id = ?",
        )
        .bind(row.biblio_id)
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
        .bind(row.biblio_id)
        .fetch_all(&state.pool)
        .await?;
        Some(rows)
    } else {
        None
    };

    let items = if includes.contains("items") {
        let rows = sqlx::query_as::<_, ItemSummary>(
            "SELECT item_id, item_code, call_number, coll_type_id, location_id, item_status_id, last_update FROM item WHERE biblio_id = ? ORDER BY item_id DESC",
        )
        .bind(row.biblio_id)
        .fetch_all(&state.pool)
        .await?;
        Some(rows)
    } else {
        None
    };

    let attachments = if includes.contains("attachments") || includes.contains("files") {
        let rows = sqlx::query_as::<_, AttachmentInfo>(
            "SELECT f.file_id, f.file_title, f.file_name, f.file_url, f.file_dir, f.mime_type, ba.placement, ba.access_type, ba.access_limit FROM biblio_attachment ba JOIN files f ON f.file_id = ba.file_id WHERE ba.biblio_id = ? ORDER BY ba.file_id DESC",
        )
        .bind(row.biblio_id)
        .fetch_all(&state.pool)
        .await?;
        Some(rows)
    } else {
        None
    };

    let custom = if includes.contains("custom") {
        if let Some(row) = sqlx::query("SELECT * FROM biblio_custom WHERE biblio_id = ?")
            .bind(row.biblio_id)
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

    let relations = if includes.contains("relations") {
        let rows = sqlx::query_as::<_, BiblioRelationInfo>(
            "SELECT br.rel_biblio_id AS biblio_id, b.title, br.rel_type FROM biblio_relation br JOIN biblio b ON b.biblio_id = br.rel_biblio_id WHERE br.biblio_id = ?",
        )
        .bind(row.biblio_id)
        .fetch_all(&state.pool)
        .await?;
        Some(rows)
    } else {
        None
    };

    Ok(Json(BiblioResponse {
        biblio: row,
        gmd,
        publisher,
        language,
        content_type,
        media_type,
        carrier_type,
        frequency,
        place,
        authors,
        topics,
        items,
        relations,
        attachments,
        custom,
    }))
}

fn row_to_json(row: &MySqlRow) -> JsonValue {
    let mut map = serde_json::Map::new();
    for (idx, col) in row.columns().iter().enumerate() {
        let key = col.name().to_string();
        let val: Option<String> = row.try_get(idx).ok();
        map.insert(
            key,
            val.map(JsonValue::String).unwrap_or(JsonValue::Null),
        );
    }
    JsonValue::Object(map)
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

    let rec = sqlx::query_as::<_, Biblio>("SELECT biblio_id, title, gmd_id, publisher_id, publish_year, language_id, content_type_id, media_type_id, carrier_type_id, frequency_id, publish_place_id, classification, call_number, opac_hide, promoted, input_date, last_update FROM biblio WHERE biblio_id = ?")
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

    let rec = sqlx::query_as::<_, Biblio>("SELECT biblio_id, title, gmd_id, publisher_id, publish_year, language_id, content_type_id, media_type_id, carrier_type_id, frequency_id, publish_place_id, classification, call_number, opac_hide, promoted, input_date, last_update FROM biblio WHERE biblio_id = ?")
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
