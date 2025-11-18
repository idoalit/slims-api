use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Serialize;
use sqlx::{FromRow, mysql::MySqlRow};

use crate::{
    auth::{AuthUser, ModuleAccess, Permission},
    config::AppState,
    error::AppError,
    resources::{PagedResponse, Pagination},
};

#[derive(Debug, Serialize, FromRow)]
pub struct MemberType {
    pub member_type_id: i64,
    pub member_type_name: String,
    pub loan_limit: i64,
    pub loan_periode: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CollType {
    pub coll_type_id: i64,
    pub coll_type_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Location {
    pub location_id: String,
    pub location_name: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Language {
    pub language_id: String,
    pub language_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Gmd {
    pub gmd_id: i64,
    pub gmd_code: Option<String>,
    pub gmd_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ItemStatus {
    pub item_status_id: String,
    pub item_status_name: String,
    pub no_loan: i16,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Frequency {
    pub frequency_id: i64,
    pub frequency: String,
    pub language_prefix: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Module {
    pub module_id: i64,
    pub module_name: String,
    pub module_path: Option<String>,
    pub module_desc: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Place {
    pub place_id: i64,
    pub place_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Publisher {
    pub publisher_id: i64,
    pub publisher_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Supplier {
    pub supplier_id: i64,
    pub supplier_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Topic {
    pub topic_id: i64,
    pub topic: String,
    pub topic_type: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ContentType {
    pub id: i64,
    pub content_type: String,
    pub code: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MediaType {
    pub id: i64,
    pub media_type: String,
    pub code: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CarrierType {
    pub id: i64,
    pub carrier_type: String,
    pub code: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct RelationTerm {
    pub rt_id: String,
    pub rt_desc: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct LoanRule {
    pub loan_rules_id: i64,
    pub member_type_id: i64,
    pub coll_type_id: i64,
    pub loan_limit: i64,
    pub loan_periode: i64,
}

async fn paged_lookup<T>(
    state: &AppState,
    pagination: Pagination,
    data_query: &str,
    count_query: &str,
) -> Result<PagedResponse<T>, AppError>
where
    for<'r> T: FromRow<'r, MySqlRow> + Send + Unpin,
{
    let (limit, offset, page, per_page) = pagination.limit_offset();
    let total: i64 = sqlx::query_scalar(count_query)
        .fetch_one(&state.pool)
        .await?;

    let data = sqlx::query_as::<_, T>(data_query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    Ok(PagedResponse {
        data,
        page,
        per_page,
        total,
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/member-types", get(member_types))
        .route("/coll-types", get(coll_types))
        .route("/locations", get(locations))
        .route("/languages", get(languages))
        .route("/gmd", get(gmds))
        .route("/item-statuses", get(item_statuses))
        .route("/frequencies", get(frequencies))
        .route("/modules", get(modules))
        .route("/places", get(places))
        .route("/publishers", get(publishers))
        .route("/suppliers", get(suppliers))
        .route("/topics", get(topics))
        .route("/content-types", get(content_types))
        .route("/media-types", get(media_types))
        .route("/carrier-types", get(carrier_types))
        .route("/relation-terms", get(relation_terms))
        .route("/loan-rules", get(loan_rules))
}

async fn member_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<MemberType>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT member_type_id, member_type_name, loan_limit, loan_periode FROM mst_member_type ORDER BY member_type_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_member_type",
    )
    .await?;

    Ok(Json(response))
}

async fn coll_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<CollType>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT coll_type_id, coll_type_name FROM mst_coll_type ORDER BY coll_type_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_coll_type",
    )
    .await?;

    Ok(Json(response))
}

async fn locations(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<Location>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT location_id, location_name FROM mst_location ORDER BY location_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_location",
    )
    .await?;

    Ok(Json(response))
}

async fn languages(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<Language>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT language_id, language_name FROM mst_language ORDER BY language_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_language",
    )
    .await?;

    Ok(Json(response))
}

async fn gmds(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<Gmd>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT gmd_id, gmd_code, gmd_name FROM mst_gmd ORDER BY gmd_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_gmd",
    )
    .await?;

    Ok(Json(response))
}

async fn item_statuses(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<ItemStatus>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT item_status_id, item_status_name, no_loan FROM mst_item_status ORDER BY item_status_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_item_status",
    )
    .await?;

    Ok(Json(response))
}

async fn frequencies(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<Frequency>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT frequency_id, frequency, language_prefix FROM mst_frequency ORDER BY frequency_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_frequency",
    )
    .await?;

    Ok(Json(response))
}

async fn modules(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<Module>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT module_id, module_name, module_path, module_desc FROM mst_module ORDER BY module_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_module",
    )
    .await?;

    Ok(Json(response))
}

async fn places(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<Place>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT place_id, place_name FROM mst_place ORDER BY place_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_place",
    )
    .await?;

    Ok(Json(response))
}

async fn publishers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<Publisher>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT publisher_id, publisher_name FROM mst_publisher ORDER BY publisher_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_publisher",
    )
    .await?;

    Ok(Json(response))
}

async fn suppliers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<Supplier>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT supplier_id, supplier_name FROM mst_supplier ORDER BY supplier_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_supplier",
    )
    .await?;

    Ok(Json(response))
}

async fn topics(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<Topic>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT topic_id, topic, topic_type FROM mst_topic ORDER BY topic_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_topic",
    )
    .await?;

    Ok(Json(response))
}

async fn content_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<ContentType>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT id, content_type, code FROM mst_content_type ORDER BY id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_content_type",
    )
    .await?;

    Ok(Json(response))
}

async fn media_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<MediaType>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT id, media_type, code FROM mst_media_type ORDER BY id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_media_type",
    )
    .await?;

    Ok(Json(response))
}

async fn carrier_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<CarrierType>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT id, carrier_type, code FROM mst_carrier_type ORDER BY id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_carrier_type",
    )
    .await?;

    Ok(Json(response))
}

async fn relation_terms(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<RelationTerm>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT rt_id, rt_desc FROM mst_relation_term ORDER BY rt_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_relation_term",
    )
    .await?;

    Ok(Json(response))
}

async fn loan_rules(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<PagedResponse<LoanRule>>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let response = paged_lookup(
        &state,
        pagination,
        "SELECT loan_rules_id, member_type_id, coll_type_id, loan_limit, loan_periode FROM mst_loan_rules ORDER BY loan_rules_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_loan_rules",
    )
    .await?;

    Ok(Json(response))
}
