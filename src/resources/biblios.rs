use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::mysql::MySqlRow;
use sqlx::{Column, FromRow, Row};
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;

use crate::{
    auth::{AuthUser, ModuleAccess, Permission},
    config::AppState,
    error::AppError,
    jsonapi::{
        JsonApiDocument, collection_document, pagination_meta, resource, resource_with_fields,
        single_document,
    },
    resources::{
        FilterField, FilterOperator, FilterValueType, ListParams, SortField, bind_filters_to_query,
        bind_filters_to_scalar, where_clause,
    },
};

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Biblio {
    pub biblio_id: i64,
    pub title: String,
    pub sor: Option<String>,
    pub edition: Option<String>,
    pub isbn_issn: Option<String>,
    pub gmd_id: Option<i32>,
    pub publisher_id: Option<i32>,
    pub publish_year: Option<String>,
    pub collation: Option<String>,
    pub series_title: Option<String>,
    pub language_id: Option<String>,
    pub source: Option<String>,
    pub content_type_id: Option<i32>,
    pub media_type_id: Option<i32>,
    pub carrier_type_id: Option<i32>,
    pub frequency_id: Option<i32>,
    pub publish_place_id: Option<i32>,
    pub classification: Option<String>,
    pub call_number: Option<String>,
    pub notes: Option<String>,
    pub image: Option<String>,
    pub file_att: Option<String>,
    pub opac_hide: Option<i16>,
    pub promoted: Option<i16>,
    pub labels: Option<String>,
    pub spec_detail_info: Option<String>,
    pub input_date: Option<NaiveDateTime>,
    pub last_update: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BiblioAuthorInput {
    pub author_id: i64,
    pub authority_level_id: u8,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BiblioTopicInput {
    pub topic_id: i64,
    pub subject_level_id: u8,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertBiblio {
    pub title: String,
    pub sor: Option<String>,
    pub edition: Option<String>,
    pub isbn_issn: Option<String>,
    pub gmd_id: Option<i32>,
    pub publisher_id: Option<i32>,
    pub publish_year: Option<String>,
    pub collation: Option<String>,
    pub series_title: Option<String>,
    pub language_id: Option<String>,
    pub source: Option<String>,
    pub content_type_id: Option<i32>,
    pub media_type_id: Option<i32>,
    pub carrier_type_id: Option<i32>,
    pub frequency_id: Option<i32>,
    pub publish_place_id: Option<i32>,
    pub classification: Option<String>,
    pub call_number: Option<String>,
    pub notes: Option<String>,
    pub image: Option<String>,
    pub file_att: Option<String>,
    pub opac_hide: Option<i16>,
    pub promoted: Option<i16>,
    pub labels: Option<String>,
    pub spec_detail_info: Option<String>,
    pub authors: Option<Vec<BiblioAuthorInput>>,
    /// Legacy input. New clients should send `authors` with an authority level.
    pub author_ids: Option<Vec<i64>>,
    pub topics: Option<Vec<BiblioTopicInput>>,
    /// Legacy input. New clients should send `topics` with a subject level.
    pub topic_ids: Option<Vec<i64>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct GmdInfo {
    pub gmd_id: i64,
    pub gmd_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct PublisherInfo {
    pub publisher_id: i64,
    pub publisher_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct LanguageInfo {
    pub language_id: String,
    pub language_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct ContentTypeInfo {
    pub id: i64,
    pub content_type: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct MediaTypeInfo {
    pub id: i64,
    pub media_type: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct CarrierTypeInfo {
    pub id: i64,
    pub carrier_type: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct FrequencyInfo {
    pub frequency_id: i64,
    pub frequency: String,
    pub language_prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct PlaceInfo {
    pub place_id: i64,
    pub place_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct ItemSummary {
    pub item_id: i64,
    pub item_code: Option<String>,
    pub call_number: Option<String>,
    pub coll_type_id: Option<i32>,
    pub location_id: Option<String>,
    pub item_status_id: Option<String>,
    pub last_update: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicItemSummary {
    pub item_id: i64,
    pub item_code: Option<String>,
    pub call_number: Option<String>,
    pub coll_type_id: Option<i32>,
    pub location_id: Option<String>,
    pub item_status_id: Option<String>,
}

impl From<ItemSummary> for PublicItemSummary {
    fn from(item: ItemSummary) -> Self {
        Self {
            item_id: item.item_id,
            item_code: item.item_code,
            call_number: item.call_number,
            coll_type_id: item.coll_type_id,
            location_id: item.location_id,
            item_status_id: item.item_status_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
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

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct BiblioRelationInfo {
    pub biblio_id: i64,
    pub title: String,
    pub rel_type: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct AuthorInfo {
    pub author_id: i64,
    pub author_name: String,
    pub authority_type: Option<String>,
    pub authority_level_id: u8,
    pub authority_level_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct TopicInfo {
    pub topic_id: i64,
    pub topic: String,
    pub topic_type: String,
    pub subject_level_id: u8,
    pub subject_level_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
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
    #[schema(value_type = Object)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<JsonValue>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicBiblio {
    pub biblio_id: i64,
    pub title: String,
    pub sor: Option<String>,
    pub edition: Option<String>,
    pub isbn_issn: Option<String>,
    pub gmd_id: Option<i32>,
    pub publisher_id: Option<i32>,
    pub publish_year: Option<String>,
    pub collation: Option<String>,
    pub series_title: Option<String>,
    pub language_id: Option<String>,
    pub source: Option<String>,
    pub content_type_id: Option<i32>,
    pub media_type_id: Option<i32>,
    pub carrier_type_id: Option<i32>,
    pub frequency_id: Option<i32>,
    pub publish_place_id: Option<i32>,
    pub classification: Option<String>,
    pub call_number: Option<String>,
    pub notes: Option<String>,
    pub image: Option<String>,
    pub labels: Option<String>,
    pub spec_detail_info: Option<String>,
}

impl From<Biblio> for PublicBiblio {
    fn from(biblio: Biblio) -> Self {
        Self {
            biblio_id: biblio.biblio_id,
            title: biblio.title,
            sor: biblio.sor,
            edition: biblio.edition,
            isbn_issn: biblio.isbn_issn,
            gmd_id: biblio.gmd_id,
            publisher_id: biblio.publisher_id,
            publish_year: biblio.publish_year,
            collation: biblio.collation,
            series_title: biblio.series_title,
            language_id: biblio.language_id,
            source: biblio.source,
            content_type_id: biblio.content_type_id,
            media_type_id: biblio.media_type_id,
            carrier_type_id: biblio.carrier_type_id,
            frequency_id: biblio.frequency_id,
            publish_place_id: biblio.publish_place_id,
            classification: biblio.classification,
            call_number: biblio.call_number,
            notes: biblio.notes,
            image: biblio.image,
            labels: biblio.labels,
            spec_detail_info: biblio.spec_detail_info,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicBiblioResponse {
    #[serde(flatten)]
    pub biblio: PublicBiblio,
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
    pub items: Option<Vec<PublicItemSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relations: Option<Vec<BiblioRelationInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<AttachmentInfo>>,
}

impl From<BiblioResponse> for PublicBiblioResponse {
    fn from(response: BiblioResponse) -> Self {
        Self {
            biblio: response.biblio.into(),
            gmd: response.gmd,
            publisher: response.publisher,
            language: response.language,
            content_type: response.content_type,
            media_type: response.media_type,
            carrier_type: response.carrier_type,
            frequency: response.frequency,
            place: response.place,
            authors: response.authors,
            topics: response.topics,
            items: response
                .items
                .map(|items| items.into_iter().map(PublicItemSummary::from).collect()),
            relations: response.relations,
            attachments: response.attachments,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CatalogVisibility {
    Protected,
    Public,
}

const PUBLIC_INCLUDES: &[&str] = &[
    "gmd",
    "publisher",
    "language",
    "content_type",
    "media_type",
    "carrier_type",
    "frequency",
    "place",
    "authors",
    "topics",
    "items",
    "relations",
    "attachments",
    "files",
];

const BIBLIO_SELECT_COLUMNS: &str = "biblio_id, title, sor, edition, isbn_issn, gmd_id, publisher_id, publish_year, collation, series_title, language_id, source, content_type_id, media_type_id, carrier_type_id, frequency_id, publish_place_id, classification, call_number, notes, image, file_att, opac_hide, promoted, labels, spec_detail_info, input_date, last_update";
const BIBLIO_SELECT_COLUMNS_QUALIFIED: &str = "b.biblio_id, b.title, b.sor, b.edition, b.isbn_issn, b.gmd_id, b.publisher_id, b.publish_year, b.collation, b.series_title, b.language_id, b.source, b.content_type_id, b.media_type_id, b.carrier_type_id, b.frequency_id, b.publish_place_id, b.classification, b.call_number, b.notes, b.image, b.file_att, b.opac_hide, b.promoted, b.labels, b.spec_detail_info, b.input_date, b.last_update";

const BIBLIO_SORTS: &[SortField<'_>] = &[
    SortField::new("biblio_id", "biblio.biblio_id"),
    SortField::new("title", "biblio.title"),
    SortField::new("input_date", "biblio.input_date"),
    SortField::new("last_update", "biblio.last_update"),
];

const BIBLIO_FILTERS: &[FilterField<'_>] = &[
    FilterField::new(
        "title",
        "biblio.title",
        FilterOperator::Like,
        FilterValueType::Text,
    ),
    FilterField::new(
        "gmd_id",
        "biblio.gmd_id",
        FilterOperator::Equals,
        FilterValueType::Integer,
    ),
    FilterField::new(
        "language_id",
        "biblio.language_id",
        FilterOperator::Equals,
        FilterValueType::Text,
    ),
];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_biblios).post(create_biblio))
        .route("/search", get(simple_search_biblios))
        .route("/search/advanced", post(advanced_search_biblios))
        .route(
            "/:biblio_id",
            get(get_biblio).put(update_biblio).delete(delete_biblio),
        )
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_public_biblios))
        .route("/search", get(search_public_biblios))
        .route("/:biblio_id", get(get_public_biblio))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SimpleSearchParams {
    pub q: String,
    #[serde(flatten)]
    pub list: ListParams,
}

#[derive(Debug, Deserialize, Clone, Copy, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BooleanOp {
    And,
    Or,
}

impl Default for BooleanOp {
    fn default() -> Self {
        BooleanOp::And
    }
}

impl BooleanOp {
    fn as_sql(&self) -> &'static str {
        match self {
            BooleanOp::And => "AND",
            BooleanOp::Or => "OR",
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    Contains,
    Exact,
    StartsWith,
    EndsWith,
}

impl Default for MatchType {
    fn default() -> Self {
        MatchType::Contains
    }
}

#[derive(Debug, Deserialize, Clone, Copy, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchField {
    Title,
    Author,
    Topic,
    Publisher,
    IsbnIssn,
    CallNumber,
    Classification,
}

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct AdvancedClause {
    pub field: SearchField,
    pub value: String,
    #[serde(default)]
    pub op: BooleanOp,
    #[serde(default)]
    pub r#type: MatchType,
}

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct AdvancedSearchPayload {
    pub clauses: Vec<AdvancedClause>,
    #[serde(flatten)]
    pub list: ListParams,
}

async fn enrich_biblios(
    state: &AppState,
    includes: &HashSet<String>,
    rows: Vec<Biblio>,
    visibility: CatalogVisibility,
) -> Result<Vec<BiblioResponse>, AppError> {
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
        let custom = if visibility == CatalogVisibility::Protected && includes.contains("custom") {
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
                "SELECT a.author_id, a.author_name, a.authority_type, ba.level AS authority_level_id, al.authority_level_name FROM biblio_author ba JOIN mst_author a ON ba.author_id = a.author_id JOIN mst_authority_level al ON ba.level = al.authority_level_id WHERE ba.biblio_id = ? ORDER BY ba.level, a.author_name",
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
                "SELECT t.topic_id, t.topic, t.topic_type, bt.level AS subject_level_id, sl.subject_level_name FROM biblio_topic bt JOIN mst_topic t ON bt.topic_id = t.topic_id JOIN mst_subject_level sl ON bt.level = sl.subject_level_id WHERE bt.biblio_id = ? ORDER BY bt.level, t.topic",
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
            let rows = sqlx::query_as::<_, AttachmentInfo>(attachment_query(visibility))
                .bind(biblio.biblio_id)
                .fetch_all(&state.pool)
                .await?;
            Some(rows)
        } else {
            None
        };

        let relations = if includes.contains("relations") {
            let rows = sqlx::query_as::<_, BiblioRelationInfo>(relation_query(visibility))
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

    Ok(data)
}

fn attachment_query(visibility: CatalogVisibility) -> &'static str {
    match visibility {
        CatalogVisibility::Protected => {
            "SELECT f.file_id, f.file_title, f.file_name, f.file_url, f.file_dir, f.mime_type, ba.placement, ba.access_type, ba.access_limit FROM biblio_attachment ba JOIN files f ON f.file_id = ba.file_id WHERE ba.biblio_id = ? ORDER BY ba.file_id DESC"
        }
        CatalogVisibility::Public => {
            "SELECT f.file_id, f.file_title, f.file_name, f.file_url, f.file_dir, f.mime_type, ba.placement, ba.access_type, ba.access_limit FROM biblio_attachment ba JOIN files f ON f.file_id = ba.file_id WHERE ba.biblio_id = ? AND ba.access_type = 'public' ORDER BY ba.file_id DESC"
        }
    }
}

fn relation_query(visibility: CatalogVisibility) -> &'static str {
    match visibility {
        CatalogVisibility::Protected => {
            "SELECT br.rel_biblio_id AS biblio_id, b.title, br.rel_type FROM biblio_relation br JOIN biblio b ON b.biblio_id = br.rel_biblio_id WHERE br.biblio_id = ?"
        }
        CatalogVisibility::Public => {
            "SELECT br.rel_biblio_id AS biblio_id, b.title, br.rel_type FROM biblio_relation br JOIN biblio b ON b.biblio_id = br.rel_biblio_id WHERE br.biblio_id = ? AND COALESCE(b.opac_hide, 0) = 0"
        }
    }
}

async fn fetch_biblio_response(
    state: &AppState,
    biblio_id: i64,
    includes: &HashSet<String>,
    visibility: CatalogVisibility,
) -> Result<BiblioResponse, AppError> {
    let sql = match visibility {
        CatalogVisibility::Protected => {
            format!("SELECT {BIBLIO_SELECT_COLUMNS} FROM biblio WHERE biblio_id = ?")
        }
        CatalogVisibility::Public => {
            format!(
                "SELECT {BIBLIO_SELECT_COLUMNS} FROM biblio WHERE biblio_id = ? AND COALESCE(opac_hide, 0) = 0"
            )
        }
    };
    let row = sqlx::query_as::<_, Biblio>(&sql)
        .bind(biblio_id)
        .fetch_one(&state.pool)
        .await?;

    enrich_biblios(state, includes, vec![row], visibility)
        .await?
        .pop()
        .ok_or(AppError::NotFound)
}

fn validate_public_includes(includes: &HashSet<String>) -> Result<(), AppError> {
    if let Some(include) = includes
        .iter()
        .find(|include| !PUBLIC_INCLUDES.contains(&include.as_str()))
    {
        return Err(AppError::BadRequest(format!(
            "include `{include}` is not supported for the public catalog"
        )));
    }
    Ok(())
}

fn public_where_clause(filters: &[crate::resources::FilterClause]) -> String {
    let filter_sql = where_clause(filters);
    if filter_sql.is_empty() {
        "WHERE COALESCE(biblio.opac_hide, 0) = 0".to_string()
    } else {
        format!(
            "WHERE COALESCE(biblio.opac_hide, 0) = 0 AND {}",
            filter_sql.trim_start_matches("WHERE ")
        )
    }
}

#[utoipa::path(
    get,
    path = "/catalog/biblios",
    responses(
        (status = 200, description = "Public OPAC bibliography list", body = JsonApiDocument),
        (status = 400, description = "Invalid query parameter"),
    ),
    tag = "Catalog"
)]
async fn list_public_biblios(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    let pagination = params.pagination();
    let includes = params.includes();
    validate_public_includes(&includes)?;
    let biblio_fields = params.fieldset("biblios");
    let (limit, offset, page, per_page) = pagination.limit_offset();
    let sort_clause = params.sort_clause(BIBLIO_SORTS, "biblio.biblio_id DESC")?;
    let filters = params.filter_clauses(BIBLIO_FILTERS)?;
    let where_sql = public_where_clause(&filters);

    let count_sql = format!("SELECT COUNT(*) FROM biblio {where_sql}");
    let total = bind_filters_to_scalar(sqlx::query_scalar::<_, i64>(&count_sql), &filters)
        .fetch_one(&state.pool)
        .await?;

    let data_sql = format!(
        "SELECT {BIBLIO_SELECT_COLUMNS} FROM biblio {where_sql} ORDER BY {sort_clause} LIMIT ? OFFSET ?"
    );
    let rows = bind_filters_to_query(sqlx::query_as::<_, Biblio>(&data_sql), &filters)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let data = enrich_biblios(&state, &includes, rows, CatalogVisibility::Public).await?;
    let documents = data
        .into_iter()
        .map(|response| {
            let id = response.biblio.biblio_id.to_string();
            resource_with_fields(
                "biblios",
                id,
                PublicBiblioResponse::from(response),
                biblio_fields,
            )
        })
        .collect();

    Ok(Json(collection_document(
        documents,
        pagination_meta(page, per_page, total),
    )))
}

#[utoipa::path(
    get,
    path = "/catalog/biblios/search",
    params(("q" = String, Query, description = "Public catalog search keyword", example = "rust")),
    responses(
        (status = 200, description = "Public OPAC bibliography search results", body = JsonApiDocument),
        (status = 400, description = "Empty query or unsupported include"),
    ),
    tag = "Catalog"
)]
async fn search_public_biblios(
    State(state): State<AppState>,
    Query(params): Query<SimpleSearchParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    let keyword = params.q.trim();
    if keyword.is_empty() {
        return Err(AppError::BadRequest("query cannot be empty".into()));
    }

    let pagination = params.list.pagination();
    let includes = params.list.includes();
    validate_public_includes(&includes)?;
    let biblio_fields = params.list.fieldset("biblios");
    let (limit, offset, page, per_page) = pagination.limit_offset();
    let pattern = format!("%{keyword}%");

    let ids_subquery = r#"
        SELECT biblio_id FROM biblio WHERE title LIKE ?
        UNION
        SELECT ba.biblio_id
        FROM biblio_author ba
        JOIN mst_author a ON a.author_id = ba.author_id
        WHERE a.author_name LIKE ?
        UNION
        SELECT bt.biblio_id
        FROM biblio_topic bt
        JOIN mst_topic t ON t.topic_id = bt.topic_id
        WHERE t.topic LIKE ?
    "#;

    let count_sql = format!(
        "SELECT COUNT(*) FROM biblio b JOIN ({ids_subquery}) ids ON ids.biblio_id = b.biblio_id WHERE COALESCE(b.opac_hide, 0) = 0"
    );
    let total: i64 = sqlx::query_scalar(&count_sql)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(&state.pool)
        .await?;

    let data_sql = format!(
        r#"
        SELECT {BIBLIO_SELECT_COLUMNS_QUALIFIED}
        FROM biblio b
        JOIN ({ids_subquery}) ids ON ids.biblio_id = b.biblio_id
        WHERE COALESCE(b.opac_hide, 0) = 0
        ORDER BY b.biblio_id DESC
        LIMIT ? OFFSET ?
        "#
    );
    let rows = sqlx::query_as::<_, Biblio>(&data_sql)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let data = enrich_biblios(&state, &includes, rows, CatalogVisibility::Public).await?;
    let documents = data
        .into_iter()
        .map(|response| {
            let id = response.biblio.biblio_id.to_string();
            resource_with_fields(
                "biblios",
                id,
                PublicBiblioResponse::from(response),
                biblio_fields,
            )
        })
        .collect();

    Ok(Json(collection_document(
        documents,
        pagination_meta(page, per_page, total),
    )))
}

#[utoipa::path(
    get,
    path = "/catalog/biblios/{biblio_id}",
    params(("biblio_id" = i64, Path, description = "Biblio ID")),
    responses(
        (status = 200, description = "Public OPAC bibliography detail", body = JsonApiDocument),
        (status = 400, description = "Unsupported include"),
        (status = 404, description = "Bibliography is missing or hidden from OPAC"),
    ),
    tag = "Catalog"
)]
async fn get_public_biblio(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
    Path(biblio_id): Path<i64>,
) -> Result<Json<JsonApiDocument>, AppError> {
    let includes = params.includes();
    validate_public_includes(&includes)?;
    let biblio_fields = params.fieldset("biblios");

    let response =
        fetch_biblio_response(&state, biblio_id, &includes, CatalogVisibility::Public).await?;
    let id = response.biblio.biblio_id.to_string();

    Ok(Json(single_document(resource_with_fields(
        "biblios",
        id,
        PublicBiblioResponse::from(response),
        biblio_fields,
    ))))
}

#[utoipa::path(
    get,
    path = "/biblios",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Biblios"
)]
async fn list_biblios(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Read)?;

    let pagination = params.pagination();
    let includes = params.includes();
    let biblio_fields = params.fieldset("biblios");
    let (limit, offset, page, per_page) = pagination.limit_offset();
    let sort_clause = params.sort_clause(BIBLIO_SORTS, "biblio.biblio_id DESC")?;
    let filters = params.filter_clauses(BIBLIO_FILTERS)?;
    let where_sql = where_clause(&filters);

    let count_sql = format!("SELECT COUNT(*) FROM biblio {}", where_sql);
    let total = bind_filters_to_scalar(sqlx::query_scalar::<_, i64>(&count_sql), &filters)
        .fetch_one(&state.pool)
        .await?;

    let data_sql = format!(
        "SELECT {BIBLIO_SELECT_COLUMNS} FROM biblio {} ORDER BY {} LIMIT ? OFFSET ?",
        where_sql, sort_clause
    );
    let rows = bind_filters_to_query(sqlx::query_as::<_, Biblio>(&data_sql), &filters)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let data = enrich_biblios(&state, &includes, rows, CatalogVisibility::Protected).await?;
    let documents = data
        .into_iter()
        .map(|biblio| {
            resource_with_fields(
                "biblios",
                biblio.biblio.biblio_id.to_string(),
                biblio,
                biblio_fields,
            )
        })
        .collect();

    Ok(Json(collection_document(
        documents,
        pagination_meta(page, per_page, total),
    )))
}

#[utoipa::path(
    get,
    path = "/biblios/search",
    params(("q" = String, Query, description = "Kata kunci pencarian", example = "rust")),
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Biblios"
)]
async fn simple_search_biblios(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<SimpleSearchParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Read)?;

    let keyword = params.q.trim();
    if keyword.is_empty() {
        return Err(AppError::BadRequest("query cannot be empty".into()));
    }

    let pagination = params.list.pagination();
    let includes = params.list.includes();
    let biblio_fields = params.list.fieldset("biblios");
    let (limit, offset, page, per_page) = pagination.limit_offset();
    let pattern = format!("%{}%", keyword);

    // Use UNION to avoid OR, so each branch can leverage its own index.
    let ids_subquery = r#"
        SELECT biblio_id FROM biblio WHERE title LIKE ?
        UNION
        SELECT ba.biblio_id
        FROM biblio_author ba
        JOIN mst_author a ON a.author_id = ba.author_id
        WHERE a.author_name LIKE ?
        UNION
        SELECT bt.biblio_id
        FROM biblio_topic bt
        JOIN mst_topic t ON t.topic_id = bt.topic_id
        WHERE t.topic LIKE ?
    "#;

    let count_sql = format!("SELECT COUNT(*) FROM ({}) ids", ids_subquery);
    let total: i64 = sqlx::query_scalar(&count_sql)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(&state.pool)
        .await?;

    let data_sql = format!(
        r#"
        SELECT {BIBLIO_SELECT_COLUMNS_QUALIFIED}
        FROM biblio b
        JOIN ({}) ids ON ids.biblio_id = b.biblio_id
        ORDER BY b.biblio_id DESC
        LIMIT ? OFFSET ?
        "#,
        ids_subquery
    );

    let rows = sqlx::query_as::<_, Biblio>(&data_sql)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let data = enrich_biblios(&state, &includes, rows, CatalogVisibility::Protected).await?;
    let documents = data
        .into_iter()
        .map(|biblio| {
            resource_with_fields(
                "biblios",
                biblio.biblio.biblio_id.to_string(),
                biblio,
                biblio_fields,
            )
        })
        .collect();

    Ok(Json(collection_document(
        documents,
        pagination_meta(page, per_page, total),
    )))
}

fn match_pattern(value: &str, matcher: MatchType) -> String {
    match matcher {
        MatchType::Contains => format!("%{}%", value),
        MatchType::Exact => value.to_string(),
        MatchType::StartsWith => format!("{}%", value),
        MatchType::EndsWith => format!("%{}", value),
    }
}

#[utoipa::path(
    post,
    path = "/biblios/search/advanced",
    request_body = AdvancedSearchPayload,
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Biblios"
)]
async fn advanced_search_biblios(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<AdvancedSearchPayload>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Read)?;

    let clauses: Vec<&AdvancedClause> = payload
        .clauses
        .iter()
        .filter(|clause| !clause.value.trim().is_empty())
        .collect();

    if clauses.is_empty() {
        return Err(AppError::BadRequest("clauses cannot be empty".into()));
    }

    let pagination = payload.list.pagination();
    let includes = payload.list.includes();
    let biblio_fields = payload.list.fieldset("biblios");
    let (limit, offset, page, per_page) = pagination.limit_offset();

    let mut joins = String::new();
    let mut joined_authors = false;
    let mut joined_topics = false;
    let mut joined_publishers = false;
    let mut conditions: Vec<String> = Vec::with_capacity(clauses.len());
    let mut bindings: Vec<String> = Vec::with_capacity(clauses.len());

    for clause in clauses {
        let column = match clause.field {
            SearchField::Title => "b.title",
            SearchField::Author => {
                if !joined_authors {
                    joins.push_str(
                        " LEFT JOIN biblio_author ba ON ba.biblio_id = b.biblio_id LEFT JOIN mst_author a ON a.author_id = ba.author_id",
                    );
                    joined_authors = true;
                }
                "a.author_name"
            }
            SearchField::Topic => {
                if !joined_topics {
                    joins.push_str(
                        " LEFT JOIN biblio_topic bt ON bt.biblio_id = b.biblio_id LEFT JOIN mst_topic t ON t.topic_id = bt.topic_id",
                    );
                    joined_topics = true;
                }
                "t.topic"
            }
            SearchField::Publisher => {
                if !joined_publishers {
                    joins.push_str(" LEFT JOIN mst_publisher p ON p.publisher_id = b.publisher_id");
                    joined_publishers = true;
                }
                "p.publisher_name"
            }
            SearchField::IsbnIssn => "b.isbn_issn",
            SearchField::CallNumber => "b.call_number",
            SearchField::Classification => "b.classification",
        };

        let pattern = match_pattern(clause.value.trim(), clause.r#type);
        let prefix = if conditions.is_empty() {
            ""
        } else {
            clause.op.as_sql()
        };

        if prefix.is_empty() {
            conditions.push(format!("{} LIKE ?", column));
        } else {
            conditions.push(format!("{} {} LIKE ?", prefix, column));
        }

        bindings.push(pattern);
    }

    let where_clause = format!(" WHERE {}", conditions.join(" "));
    let base_from = format!(" FROM biblio b{}", joins);

    let count_sql = format!(
        "SELECT COUNT(DISTINCT b.biblio_id){}{}",
        base_from, where_clause
    );
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for value in &bindings {
        count_query = count_query.bind(value);
    }
    let total = count_query.fetch_one(&state.pool).await?;

    let data_sql = format!(
        "SELECT DISTINCT {BIBLIO_SELECT_COLUMNS_QUALIFIED}{}{} ORDER BY b.biblio_id DESC LIMIT ? OFFSET ?",
        base_from, where_clause
    );
    let mut data_query = sqlx::query_as::<_, Biblio>(&data_sql);
    for value in &bindings {
        data_query = data_query.bind(value);
    }
    let rows = data_query
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let data = enrich_biblios(&state, &includes, rows, CatalogVisibility::Protected).await?;
    let documents = data
        .into_iter()
        .map(|biblio| {
            resource_with_fields(
                "biblios",
                biblio.biblio.biblio_id.to_string(),
                biblio,
                biblio_fields,
            )
        })
        .collect();

    Ok(Json(collection_document(
        documents,
        pagination_meta(page, per_page, total),
    )))
}

#[utoipa::path(
    get,
    path = "/biblios/{biblio_id}",
    params(("biblio_id" = i64, Path, description = "Biblio ID")),
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Biblios"
)]
async fn get_biblio(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
    Path(biblio_id): Path<i64>,
    auth: AuthUser,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Read)?;

    let includes = params.includes();
    let biblio_fields = params.fieldset("biblios");
    let response =
        fetch_biblio_response(&state, biblio_id, &includes, CatalogVisibility::Protected).await?;
    let id = response.biblio.biblio_id.to_string();
    Ok(Json(single_document(resource_with_fields(
        "biblios",
        id,
        response,
        biblio_fields,
    ))))
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

async fn replace_biblio_links(
    state: &AppState,
    biblio_id: i64,
    authors: &Option<Vec<BiblioAuthorInput>>,
    author_ids: &Option<Vec<i64>>,
    topics: &Option<Vec<BiblioTopicInput>>,
    topic_ids: &Option<Vec<i64>>,
) -> Result<(), AppError> {
    let author_links = authors
        .as_ref()
        .map(|items| {
            items
                .iter()
                .map(|item| (item.author_id, item.authority_level_id))
                .collect::<HashMap<_, _>>()
        })
        .or_else(|| {
            author_ids.as_ref().map(|ids| {
                ids.iter()
                    .copied()
                    .map(|author_id| (author_id, 1))
                    .collect::<HashMap<_, _>>()
            })
        });

    if let Some(author_links) = author_links {
        for authority_level_id in author_links.values().copied().collect::<HashSet<_>>() {
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM mst_authority_level WHERE authority_level_id = ?",
            )
            .bind(authority_level_id)
            .fetch_one(&state.pool)
            .await?;
            if exists == 0 {
                return Err(AppError::BadRequest(format!(
                    "authority_level_id {authority_level_id} tidak ditemukan"
                )));
            }
        }

        sqlx::query("DELETE FROM biblio_author WHERE biblio_id = ?")
            .bind(biblio_id)
            .execute(&state.pool)
            .await?;
        for (author_id, authority_level_id) in author_links {
            sqlx::query("INSERT INTO biblio_author (biblio_id, author_id, level) VALUES (?, ?, ?)")
                .bind(biblio_id)
                .bind(author_id)
                .bind(authority_level_id)
                .execute(&state.pool)
                .await?;
        }
    }

    let topic_links = topics
        .as_ref()
        .map(|items| {
            items
                .iter()
                .map(|item| (item.topic_id, item.subject_level_id))
                .collect::<HashMap<_, _>>()
        })
        .or_else(|| {
            topic_ids.as_ref().map(|ids| {
                ids.iter()
                    .copied()
                    .map(|topic_id| (topic_id, 1))
                    .collect::<HashMap<_, _>>()
            })
        });

    if let Some(topic_links) = topic_links {
        for subject_level_id in topic_links.values().copied().collect::<HashSet<_>>() {
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM mst_subject_level WHERE subject_level_id = ?",
            )
            .bind(subject_level_id)
            .fetch_one(&state.pool)
            .await?;
            if exists == 0 {
                return Err(AppError::BadRequest(format!(
                    "subject_level_id {subject_level_id} tidak ditemukan"
                )));
            }
        }

        sqlx::query("DELETE FROM biblio_topic WHERE biblio_id = ?")
            .bind(biblio_id)
            .execute(&state.pool)
            .await?;
        for (topic_id, subject_level_id) in topic_links {
            sqlx::query("INSERT INTO biblio_topic (biblio_id, topic_id, level) VALUES (?, ?, ?)")
                .bind(biblio_id)
                .bind(topic_id)
                .bind(subject_level_id)
                .execute(&state.pool)
                .await?;
        }
    }

    Ok(())
}

fn normalized_frequency_id(frequency_id: Option<i32>) -> i32 {
    frequency_id.unwrap_or(0)
}

#[utoipa::path(
    post,
    path = "/biblios",
    request_body = UpsertBiblio,
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Biblios"
)]
async fn create_biblio(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpsertBiblio>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Write)?;

    let now = chrono::Utc::now().naive_utc();

    let result = sqlx::query(
        "INSERT INTO biblio (title, sor, edition, isbn_issn, gmd_id, publisher_id, publish_year, collation, series_title, language_id, source, content_type_id, media_type_id, carrier_type_id, frequency_id, publish_place_id, classification, call_number, notes, image, file_att, opac_hide, promoted, labels, spec_detail_info, input_date, last_update) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&payload.title)
    .bind(&payload.sor)
    .bind(&payload.edition)
    .bind(&payload.isbn_issn)
    .bind(payload.gmd_id)
    .bind(payload.publisher_id)
    .bind(&payload.publish_year)
    .bind(&payload.collation)
    .bind(&payload.series_title)
    .bind(&payload.language_id)
    .bind(&payload.source)
    .bind(payload.content_type_id)
    .bind(payload.media_type_id)
    .bind(payload.carrier_type_id)
    .bind(normalized_frequency_id(payload.frequency_id))
    .bind(payload.publish_place_id)
    .bind(&payload.classification)
    .bind(&payload.call_number)
    .bind(&payload.notes)
    .bind(&payload.image)
    .bind(&payload.file_att)
    .bind(payload.opac_hide.unwrap_or(0))
    .bind(payload.promoted.unwrap_or(0))
    .bind(&payload.labels)
    .bind(&payload.spec_detail_info)
    .bind(now)
    .bind(now)
    .execute(&state.pool)
    .await?;

    let biblio_id = result.last_insert_id() as i64;
    replace_biblio_links(
        &state,
        biblio_id,
        &payload.authors,
        &payload.author_ids,
        &payload.topics,
        &payload.topic_ids,
    )
    .await?;

    let select_sql = format!("SELECT {BIBLIO_SELECT_COLUMNS} FROM biblio WHERE biblio_id = ?");
    let rec = sqlx::query_as::<_, Biblio>(&select_sql)
        .bind(biblio_id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(single_document(resource(
        "biblios",
        rec.biblio_id.to_string(),
        rec,
    ))))
}

#[utoipa::path(
    put,
    path = "/biblios/{biblio_id}",
    params(("biblio_id" = i64, Path, description = "Biblio ID")),
    request_body = UpsertBiblio,
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Biblios"
)]
async fn update_biblio(
    State(state): State<AppState>,
    Path(biblio_id): Path<i64>,
    auth: AuthUser,
    Json(payload): Json<UpsertBiblio>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Write)?;

    let now = chrono::Utc::now().naive_utc();

    let updated = sqlx::query(
        "UPDATE biblio SET title = ?, sor = ?, edition = ?, isbn_issn = ?, gmd_id = ?, publisher_id = ?, publish_year = ?, collation = ?, series_title = ?, language_id = ?, source = ?, content_type_id = ?, media_type_id = ?, carrier_type_id = ?, frequency_id = ?, publish_place_id = ?, classification = ?, call_number = ?, notes = ?, image = ?, file_att = ?, opac_hide = ?, promoted = ?, labels = ?, spec_detail_info = ?, last_update = ? WHERE biblio_id = ?",
    )
    .bind(&payload.title)
    .bind(&payload.sor)
    .bind(&payload.edition)
    .bind(&payload.isbn_issn)
    .bind(payload.gmd_id)
    .bind(payload.publisher_id)
    .bind(&payload.publish_year)
    .bind(&payload.collation)
    .bind(&payload.series_title)
    .bind(&payload.language_id)
    .bind(&payload.source)
    .bind(payload.content_type_id)
    .bind(payload.media_type_id)
    .bind(payload.carrier_type_id)
    .bind(normalized_frequency_id(payload.frequency_id))
    .bind(payload.publish_place_id)
    .bind(&payload.classification)
    .bind(&payload.call_number)
    .bind(&payload.notes)
    .bind(&payload.image)
    .bind(&payload.file_att)
    .bind(payload.opac_hide.unwrap_or(0))
    .bind(payload.promoted.unwrap_or(0))
    .bind(&payload.labels)
    .bind(&payload.spec_detail_info)
    .bind(now)
    .bind(biblio_id)
    .execute(&state.pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    replace_biblio_links(
        &state,
        biblio_id,
        &payload.authors,
        &payload.author_ids,
        &payload.topics,
        &payload.topic_ids,
    )
    .await?;

    let select_sql = format!("SELECT {BIBLIO_SELECT_COLUMNS} FROM biblio WHERE biblio_id = ?");
    let rec = sqlx::query_as::<_, Biblio>(&select_sql)
        .bind(biblio_id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(single_document(resource(
        "biblios",
        rec.biblio_id.to_string(),
        rec,
    ))))
}

#[utoipa::path(
    delete,
    path = "/biblios/{biblio_id}",
    params(("biblio_id" = i64, Path, description = "Biblio ID")),
    responses((status = 204, description = "Biblio deleted")),
    security(("bearerAuth" = [])),
    tag = "Biblios"
)]
async fn delete_biblio(
    State(state): State<AppState>,
    Path(biblio_id): Path<i64>,
    auth: AuthUser,
) -> Result<StatusCode, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Write)?;

    sqlx::query("DELETE FROM biblio WHERE biblio_id = ?")
        .bind(biblio_id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{FilterClause, FilterValue};

    fn sample_biblio() -> Biblio {
        Biblio {
            biblio_id: 42,
            title: "Public title".into(),
            sor: Some("Author statement".into()),
            edition: Some("2nd ed.".into()),
            isbn_issn: Some("9780000000000".into()),
            gmd_id: Some(1),
            publisher_id: Some(2),
            publish_year: Some("2026".into()),
            collation: Some("x, 200 pages".into()),
            series_title: Some("Series".into()),
            language_id: Some("en".into()),
            source: Some("buy".into()),
            content_type_id: Some(3),
            media_type_id: Some(4),
            carrier_type_id: Some(5),
            frequency_id: None,
            publish_place_id: Some(6),
            classification: Some("005.13".into()),
            call_number: Some("005.13 PUB".into()),
            notes: Some("Notes".into()),
            image: Some("cover.jpg".into()),
            file_att: Some("private.pdf".into()),
            opac_hide: Some(1),
            promoted: Some(1),
            labels: Some("favorite".into()),
            spec_detail_info: Some("Scale 1:1000".into()),
            input_date: None,
            last_update: None,
        }
    }

    #[test]
    fn public_response_excludes_internal_biblio_fields() {
        let response = PublicBiblioResponse::from(BiblioResponse {
            biblio: sample_biblio(),
            gmd: None,
            publisher: None,
            language: None,
            content_type: None,
            media_type: None,
            carrier_type: None,
            frequency: None,
            place: None,
            authors: None,
            topics: None,
            items: Some(vec![ItemSummary {
                item_id: 7,
                item_code: Some("ITEM-7".into()),
                call_number: None,
                coll_type_id: None,
                location_id: None,
                item_status_id: None,
                last_update: None,
            }]),
            relations: None,
            attachments: None,
            custom: Some(serde_json::json!({ "private": true })),
        });

        let value = serde_json::to_value(response).expect("public response serializes");
        assert_eq!(value["biblio_id"], 42);
        assert_eq!(value["title"], "Public title");
        assert!(value.get("opac_hide").is_none());
        assert!(value.get("promoted").is_none());
        assert!(value.get("input_date").is_none());
        assert!(value.get("last_update").is_none());
        assert!(value.get("custom").is_none());
        assert_eq!(value["items"][0]["item_code"], "ITEM-7");
        assert!(value["items"][0].get("last_update").is_none());
    }

    #[test]
    fn public_include_allowlist_rejects_custom_and_unknown_values() {
        let allowed = HashSet::from([
            "authors".to_string(),
            "items".to_string(),
            "attachments".to_string(),
        ]);
        assert!(validate_public_includes(&allowed).is_ok());

        for disallowed in ["custom", "loans", "unknown"] {
            let includes = HashSet::from([disallowed.to_string()]);
            assert!(matches!(
                validate_public_includes(&includes),
                Err(AppError::BadRequest(_))
            ));
        }
    }

    #[test]
    fn public_where_clause_always_applies_opac_visibility() {
        assert_eq!(
            public_where_clause(&[]),
            "WHERE COALESCE(biblio.opac_hide, 0) = 0"
        );

        let filters = vec![FilterClause {
            statement: "biblio.title LIKE ?".into(),
            value: FilterValue::Text("%rust%".into()),
        }];
        assert_eq!(
            public_where_clause(&filters),
            "WHERE COALESCE(biblio.opac_hide, 0) = 0 AND biblio.title LIKE ?"
        );
    }

    #[test]
    fn public_relation_and_attachment_queries_apply_visibility_rules() {
        assert!(attachment_query(CatalogVisibility::Public).contains("access_type = 'public'"));
        assert!(relation_query(CatalogVisibility::Public).contains("opac_hide"));
        assert!(!attachment_query(CatalogVisibility::Protected).contains("access_type = 'public'"));
        assert!(!relation_query(CatalogVisibility::Protected).contains("opac_hide"));
    }

    #[test]
    fn empty_frequency_uses_slims_not_applicable_sentinel() {
        assert_eq!(normalized_frequency_id(None), 0);
        assert_eq!(normalized_frequency_id(Some(7)), 7);
    }
}
