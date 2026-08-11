use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, mysql::MySqlRow};
use utoipa::ToSchema;

use crate::{
    auth::{AuthUser, ModuleAccess, Permission},
    config::AppState,
    error::AppError,
    jsonapi::{JsonApiDocument, collection_document, pagination_meta, resource, single_document},
    resources::Pagination,
};

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct MemberType {
    pub member_type_id: i64,
    pub member_type_name: String,
    pub loan_limit: i64,
    pub loan_periode: i64,
    pub enable_reserve: i16,
    pub reserve_limit: i64,
    pub member_periode: i64,
    pub reborrow_limit: i64,
    pub fine_each_day: i64,
    pub grace_periode: Option<i64>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct CollType {
    pub coll_type_id: i64,
    pub coll_type_name: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Location {
    pub location_id: String,
    pub location_name: Option<String>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Language {
    pub language_id: String,
    pub language_name: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Gmd {
    pub gmd_id: i64,
    pub gmd_code: Option<String>,
    pub gmd_name: String,
    pub icon_image: Option<String>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct ItemStatus {
    pub item_status_id: String,
    pub item_status_name: String,
    pub no_loan: i16,
    pub rules: Option<String>,
    pub skip_stock_take: i16,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Frequency {
    pub frequency_id: i64,
    pub frequency: String,
    pub language_prefix: Option<String>,
    pub time_increment: Option<i16>,
    pub time_unit: Option<String>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Module {
    pub module_id: i64,
    pub module_name: String,
    pub module_path: Option<String>,
    pub module_desc: Option<String>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Place {
    pub place_id: i64,
    pub place_name: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Publisher {
    pub publisher_id: i64,
    pub publisher_name: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Author {
    pub author_id: i64,
    pub author_name: String,
    pub author_year: Option<String>,
    pub authority_type: Option<String>,
    pub auth_list: Option<String>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct AuthorityType {
    pub authority_type_id: String,
    pub authority_type_name: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct AuthorityLevel {
    pub authority_level_id: u8,
    pub authority_level_name: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Supplier {
    pub supplier_id: i64,
    pub supplier_name: String,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub phone: Option<String>,
    pub contact: Option<String>,
    pub fax: Option<String>,
    pub account: Option<String>,
    pub e_mail: Option<String>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Topic {
    pub topic_id: i64,
    pub topic: String,
    pub topic_type: String,
    pub auth_list: Option<String>,
    pub classification: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct ContentType {
    pub id: i64,
    pub content_type: String,
    pub code: String,
    pub code2: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct MediaType {
    pub id: i64,
    pub media_type: String,
    pub code: String,
    pub code2: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct CarrierType {
    pub id: i64,
    pub carrier_type: String,
    pub code: String,
    pub code2: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct RelationTerm {
    pub rt_id: String,
    pub rt_desc: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct LoanRule {
    pub loan_rules_id: i64,
    pub member_type_id: i64,
    pub coll_type_id: Option<i64>,
    pub gmd_id: Option<i64>,
    pub loan_limit: i64,
    pub loan_periode: i64,
    pub reborrow_limit: i64,
    pub fine_each_day: i64,
    pub grace_periode: i64,
}

macro_rules! lookup_payload {
    ($name:ident { $($field:ident: $ty:ty),+ $(,)? }) => {
        #[derive(Debug, Deserialize, ToSchema)]
        pub struct $name {
            $(pub $field: $ty),+
        }
    };
}

lookup_payload!(UpsertMemberType {
    member_type_name: String,
    loan_limit: i64,
    loan_periode: i64,
    enable_reserve: i16,
    reserve_limit: i64,
    member_periode: i64,
    reborrow_limit: i64,
    fine_each_day: i64,
    grace_periode: Option<i64>,
});
lookup_payload!(UpsertCollType {
    coll_type_name: String
});
lookup_payload!(UpsertLocation {
    location_id: String,
    location_name: Option<String>,
});
lookup_payload!(UpsertLanguage {
    language_id: String,
    language_name: String,
});
lookup_payload!(UpsertGmd {
    gmd_code: Option<String>,
    gmd_name: String,
    icon_image: Option<String>,
});
lookup_payload!(UpsertItemStatus {
    item_status_id: String,
    item_status_name: String,
    no_loan: i16,
    rules: Option<String>,
    skip_stock_take: i16,
});
lookup_payload!(UpsertFrequency {
    frequency: String,
    language_prefix: Option<String>,
    time_increment: Option<i16>,
    time_unit: Option<String>,
});
lookup_payload!(UpsertModule {
    module_name: String,
    module_path: Option<String>,
    module_desc: Option<String>,
});
lookup_payload!(UpsertPlace { place_name: String });
lookup_payload!(UpsertPublisher {
    publisher_name: String
});
lookup_payload!(UpsertAuthor {
    author_name: String,
    author_year: Option<String>,
    authority_type: String,
    auth_list: Option<String>,
});
lookup_payload!(UpsertAuthorityType {
    authority_type_id: String,
    authority_type_name: String,
});
lookup_payload!(UpsertAuthorityLevel {
    authority_level_name: String,
});
lookup_payload!(UpsertSupplier {
    supplier_name: String,
    address: Option<String>,
    postal_code: Option<String>,
    phone: Option<String>,
    contact: Option<String>,
    fax: Option<String>,
    account: Option<String>,
    e_mail: Option<String>,
});
lookup_payload!(UpsertTopic {
    topic: String,
    topic_type: String,
    auth_list: Option<String>,
    classification: String,
});
lookup_payload!(UpsertContentType {
    content_type: String,
    code: String,
    code2: String,
});
lookup_payload!(UpsertMediaType {
    media_type: String,
    code: String,
    code2: String,
});
lookup_payload!(UpsertCarrierType {
    carrier_type: String,
    code: String,
    code2: String,
});
lookup_payload!(UpsertRelationTerm {
    rt_id: String,
    rt_desc: String,
});
lookup_payload!(UpsertLoanRule {
    member_type_id: i64,
    coll_type_id: Option<i64>,
    gmd_id: Option<i64>,
    loan_limit: i64,
    loan_periode: i64,
    reborrow_limit: i64,
    fine_each_day: i64,
    grace_periode: i64,
});

#[derive(Debug, Default)]
struct LookupParams {
    pagination: Pagination,
    q: Option<String>,
}

impl<'de> Deserialize<'de> for LookupParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawParams {
            #[serde(rename = "page[number]", alias = "page", default)]
            page_number: Option<String>,
            #[serde(rename = "page[size]", alias = "per_page", default)]
            page_size: Option<String>,
            #[serde(default)]
            q: Option<String>,
        }

        let raw = RawParams::deserialize(deserializer)?;
        let page_number = raw
            .page_number
            .map(|value| value.parse::<u32>().map_err(serde::de::Error::custom))
            .transpose()?;
        let page_size = raw
            .page_size
            .map(|value| value.parse::<u32>().map_err(serde::de::Error::custom))
            .transpose()?;

        Ok(Self {
            pagination: Pagination {
                page_number,
                page_size,
            },
            q: raw.q,
        })
    }
}

async fn paged_lookup<T, F>(
    state: &AppState,
    pagination: Pagination,
    data_query: &str,
    count_query: &str,
    resource_type: &'static str,
    mut id_fn: F,
) -> Result<JsonApiDocument, AppError>
where
    for<'r> T: FromRow<'r, MySqlRow> + Send + Unpin + Serialize + ToSchema<'static> + 'static,
    F: FnMut(&T) -> String,
{
    let (limit, offset, page, per_page) = pagination.limit_offset();
    let total: i64 = sqlx::query_scalar(count_query)
        .fetch_one(&state.pool)
        .await?;

    let rows = sqlx::query_as::<_, T>(data_query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let data = rows
        .into_iter()
        .map(|row| {
            let id = id_fn(&row);
            resource(resource_type, id, row)
        })
        .collect();

    Ok(collection_document(
        data,
        pagination_meta(page, per_page, total),
    ))
}

async fn searchable_paged_lookup<T, F>(
    state: &AppState,
    params: LookupParams,
    data_query: &str,
    count_query: &str,
    search_columns: &str,
    resource_type: &'static str,
    mut id_fn: F,
) -> Result<JsonApiDocument, AppError>
where
    for<'r> T: FromRow<'r, MySqlRow> + Send + Unpin + Serialize + ToSchema<'static> + 'static,
    F: FnMut(&T) -> String,
{
    let (limit, offset, page, per_page) = params.pagination.limit_offset();
    let search = params
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{value}%"));
    let search_clause = format!("CONCAT_WS(' ', {search_columns}) LIKE ?");

    let (data_sql, count_sql) = if search.is_some() {
        let (select, order) = data_query
            .split_once(" ORDER BY ")
            .ok_or_else(|| AppError::Internal("invalid lookup query".into()))?;
        (
            format!("{select} WHERE {search_clause} ORDER BY {order}"),
            format!("{count_query} WHERE {search_clause}"),
        )
    } else {
        (data_query.to_string(), count_query.to_string())
    };

    let total = if let Some(pattern) = search.as_ref() {
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(pattern)
            .fetch_one(&state.pool)
            .await?
    } else {
        sqlx::query_scalar::<_, i64>(&count_sql)
            .fetch_one(&state.pool)
            .await?
    };

    let rows = if let Some(pattern) = search.as_ref() {
        sqlx::query_as::<_, T>(&data_sql)
            .bind(pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?
    } else {
        sqlx::query_as::<_, T>(&data_sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?
    };

    let data = rows
        .into_iter()
        .map(|row| {
            let id = id_fn(&row);
            resource(resource_type, id, row)
        })
        .collect();

    Ok(collection_document(
        data,
        pagination_meta(page, per_page, total),
    ))
}

async fn lookup_by_id<T>(
    state: &AppState,
    query: &str,
    id: &str,
    resource_type: &'static str,
) -> Result<JsonApiDocument, AppError>
where
    for<'r> T: FromRow<'r, MySqlRow> + Send + Unpin + Serialize + ToSchema<'static> + 'static,
{
    let row = sqlx::query_as::<_, T>(query)
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    Ok(single_document(resource(resource_type, id, row)))
}

async fn remove_lookup(state: &AppState, query: &str, id: &str) -> Result<StatusCode, AppError> {
    let result = sqlx::query(query).bind(id).execute(&state.pool).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

macro_rules! numeric_lookup_crud {
    (
        $get_fn:ident, $create_fn:ident, $update_fn:ident, $delete_fn:ident,
        $collection_path:literal, $item_path:literal, $resource_type:literal, $row:ty, $payload:ty,
        $select_one:literal, $insert:literal, $update:literal, $delete:literal,
        [$($field:ident),+ $(,)?]
    ) => {
        #[utoipa::path(get, path = $item_path, params(("id" = String, Path, description = "Resource ID")), responses((status = 200, body = JsonApiDocument)), security(("bearerAuth" = [])), tag = "Lookups")]
        async fn $get_fn(State(state): State<AppState>, auth: AuthUser, Path(id): Path<String>) -> Result<Json<JsonApiDocument>, AppError> {
            auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;
            Ok(Json(lookup_by_id::<$row>(&state, $select_one, &id, $resource_type).await?))
        }

        #[utoipa::path(post, path = $collection_path, request_body = $payload, responses((status = 200, body = JsonApiDocument)), security(("bearerAuth" = [])), tag = "Lookups")]
        async fn $create_fn(State(state): State<AppState>, auth: AuthUser, Json(payload): Json<$payload>) -> Result<Json<JsonApiDocument>, AppError> {
            auth.require_access(ModuleAccess::MasterFile, Permission::Write)?;
            let result = sqlx::query($insert)$(.bind(&payload.$field))+.execute(&state.pool).await?;
            let id = result.last_insert_id().to_string();
            Ok(Json(lookup_by_id::<$row>(&state, $select_one, &id, $resource_type).await?))
        }

        #[utoipa::path(put, path = $item_path, request_body = $payload, params(("id" = String, Path, description = "Resource ID")), responses((status = 200, body = JsonApiDocument)), security(("bearerAuth" = [])), tag = "Lookups")]
        async fn $update_fn(State(state): State<AppState>, auth: AuthUser, Path(id): Path<String>, Json(payload): Json<$payload>) -> Result<Json<JsonApiDocument>, AppError> {
            auth.require_access(ModuleAccess::MasterFile, Permission::Write)?;
            lookup_by_id::<$row>(&state, $select_one, &id, $resource_type).await?;
            sqlx::query($update)$(.bind(&payload.$field))+.bind(&id).execute(&state.pool).await?;
            Ok(Json(lookup_by_id::<$row>(&state, $select_one, &id, $resource_type).await?))
        }

        #[utoipa::path(delete, path = $item_path, params(("id" = String, Path, description = "Resource ID")), responses((status = 204, description = "Resource deleted")), security(("bearerAuth" = [])), tag = "Lookups")]
        async fn $delete_fn(State(state): State<AppState>, auth: AuthUser, Path(id): Path<String>) -> Result<StatusCode, AppError> {
            auth.require_access(ModuleAccess::MasterFile, Permission::Write)?;
            remove_lookup(&state, $delete, &id).await
        }
    };
}

macro_rules! string_lookup_crud {
    (
        $get_fn:ident, $create_fn:ident, $update_fn:ident, $delete_fn:ident,
        $collection_path:literal, $item_path:literal, $resource_type:literal, $row:ty, $payload:ty, $id_field:ident,
        $select_one:literal, $insert:literal, $update:literal, $delete:literal,
        [$($create_field:ident),+ $(,)?], [$($update_field:ident),+ $(,)?]
    ) => {
        #[utoipa::path(get, path = $item_path, params(("id" = String, Path, description = "Resource ID")), responses((status = 200, body = JsonApiDocument)), security(("bearerAuth" = [])), tag = "Lookups")]
        async fn $get_fn(State(state): State<AppState>, auth: AuthUser, Path(id): Path<String>) -> Result<Json<JsonApiDocument>, AppError> {
            auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;
            Ok(Json(lookup_by_id::<$row>(&state, $select_one, &id, $resource_type).await?))
        }

        #[utoipa::path(post, path = $collection_path, request_body = $payload, responses((status = 200, body = JsonApiDocument)), security(("bearerAuth" = [])), tag = "Lookups")]
        async fn $create_fn(State(state): State<AppState>, auth: AuthUser, Json(payload): Json<$payload>) -> Result<Json<JsonApiDocument>, AppError> {
            auth.require_access(ModuleAccess::MasterFile, Permission::Write)?;
            let id = payload.$id_field.clone();
            sqlx::query($insert)$(.bind(&payload.$create_field))+.execute(&state.pool).await?;
            Ok(Json(lookup_by_id::<$row>(&state, $select_one, &id, $resource_type).await?))
        }

        #[utoipa::path(put, path = $item_path, request_body = $payload, params(("id" = String, Path, description = "Resource ID")), responses((status = 200, body = JsonApiDocument)), security(("bearerAuth" = [])), tag = "Lookups")]
        async fn $update_fn(State(state): State<AppState>, auth: AuthUser, Path(id): Path<String>, Json(payload): Json<$payload>) -> Result<Json<JsonApiDocument>, AppError> {
            auth.require_access(ModuleAccess::MasterFile, Permission::Write)?;
            let new_id = payload.$id_field.clone();
            lookup_by_id::<$row>(&state, $select_one, &id, $resource_type).await?;
            sqlx::query($update)$(.bind(&payload.$update_field))+.bind(&id).execute(&state.pool).await?;
            Ok(Json(lookup_by_id::<$row>(&state, $select_one, &new_id, $resource_type).await?))
        }

        #[utoipa::path(delete, path = $item_path, params(("id" = String, Path, description = "Resource ID")), responses((status = 204, description = "Resource deleted")), security(("bearerAuth" = [])), tag = "Lookups")]
        async fn $delete_fn(State(state): State<AppState>, auth: AuthUser, Path(id): Path<String>) -> Result<StatusCode, AppError> {
            auth.require_access(ModuleAccess::MasterFile, Permission::Write)?;
            remove_lookup(&state, $delete, &id).await
        }
    };
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/member-types", get(member_types).post(create_member_type))
        .route(
            "/member-types/:id",
            get(get_member_type)
                .put(update_member_type)
                .delete(delete_member_type),
        )
        .route("/coll-types", get(coll_types).post(create_coll_type))
        .route(
            "/coll-types/:id",
            get(get_coll_type)
                .put(update_coll_type)
                .delete(delete_coll_type),
        )
        .route("/locations", get(locations).post(create_location))
        .route(
            "/locations/:id",
            get(get_location)
                .put(update_location)
                .delete(delete_location),
        )
        .route("/languages", get(languages).post(create_language))
        .route(
            "/languages/:id",
            get(get_language)
                .put(update_language)
                .delete(delete_language),
        )
        .route("/gmd", get(gmds).post(create_gmd))
        .route("/gmd/:id", get(get_gmd).put(update_gmd).delete(delete_gmd))
        .route(
            "/item-statuses",
            get(item_statuses).post(create_item_status),
        )
        .route(
            "/item-statuses/:id",
            get(get_item_status)
                .put(update_item_status)
                .delete(delete_item_status),
        )
        .route("/frequencies", get(frequencies).post(create_frequency))
        .route(
            "/frequencies/:id",
            get(get_frequency)
                .put(update_frequency)
                .delete(delete_frequency),
        )
        .route("/modules", get(modules).post(create_module))
        .route(
            "/modules/:id",
            get(get_module).put(update_module).delete(delete_module),
        )
        .route("/places", get(places).post(create_place))
        .route(
            "/places/:id",
            get(get_place).put(update_place).delete(delete_place),
        )
        .route("/publishers", get(publishers).post(create_publisher))
        .route(
            "/publishers/:id",
            get(get_publisher)
                .put(update_publisher)
                .delete(delete_publisher),
        )
        .route("/authors", get(authors).post(create_author))
        .route(
            "/authors/:id",
            get(get_author).put(update_author).delete(delete_author),
        )
        .route(
            "/authority-types",
            get(authority_types).post(create_authority_type),
        )
        .route(
            "/authority-types/:id",
            get(get_authority_type)
                .put(update_authority_type)
                .delete(delete_authority_type),
        )
        .route(
            "/authority-levels",
            get(authority_levels).post(create_authority_level),
        )
        .route(
            "/authority-levels/:id",
            get(get_authority_level)
                .put(update_authority_level)
                .delete(delete_authority_level),
        )
        .route("/suppliers", get(suppliers).post(create_supplier))
        .route(
            "/suppliers/:id",
            get(get_supplier)
                .put(update_supplier)
                .delete(delete_supplier),
        )
        .route("/topics", get(topics).post(create_topic))
        .route(
            "/topics/:id",
            get(get_topic).put(update_topic).delete(delete_topic),
        )
        .route(
            "/content-types",
            get(content_types).post(create_content_type),
        )
        .route(
            "/content-types/:id",
            get(get_content_type)
                .put(update_content_type)
                .delete(delete_content_type),
        )
        .route("/media-types", get(media_types).post(create_media_type))
        .route(
            "/media-types/:id",
            get(get_media_type)
                .put(update_media_type)
                .delete(delete_media_type),
        )
        .route(
            "/carrier-types",
            get(carrier_types).post(create_carrier_type),
        )
        .route(
            "/carrier-types/:id",
            get(get_carrier_type)
                .put(update_carrier_type)
                .delete(delete_carrier_type),
        )
        .route(
            "/relation-terms",
            get(relation_terms).post(create_relation_term),
        )
        .route(
            "/relation-terms/:id",
            get(get_relation_term)
                .put(update_relation_term)
                .delete(delete_relation_term),
        )
        .route("/loan-rules", get(loan_rules).post(create_loan_rule))
        .route(
            "/loan-rules/:id",
            get(get_loan_rule)
                .put(update_loan_rule)
                .delete(delete_loan_rule),
        )
}

numeric_lookup_crud!(
    get_member_type,
    create_member_type,
    update_member_type,
    delete_member_type,
    "/lookups/member-types",
    "/lookups/member-types/{id}",
    "member-types",
    MemberType,
    UpsertMemberType,
    "SELECT member_type_id, member_type_name, loan_limit, loan_periode, enable_reserve, reserve_limit, member_periode, reborrow_limit, fine_each_day, grace_periode FROM mst_member_type WHERE member_type_id = ?",
    "INSERT INTO mst_member_type (member_type_name, loan_limit, loan_periode, enable_reserve, reserve_limit, member_periode, reborrow_limit, fine_each_day, grace_periode, input_date, last_update) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURDATE(), CURDATE())",
    "UPDATE mst_member_type SET member_type_name = ?, loan_limit = ?, loan_periode = ?, enable_reserve = ?, reserve_limit = ?, member_periode = ?, reborrow_limit = ?, fine_each_day = ?, grace_periode = ?, last_update = CURDATE() WHERE member_type_id = ?",
    "DELETE FROM mst_member_type WHERE member_type_id = ?",
    [
        member_type_name,
        loan_limit,
        loan_periode,
        enable_reserve,
        reserve_limit,
        member_periode,
        reborrow_limit,
        fine_each_day,
        grace_periode
    ]
);

numeric_lookup_crud!(
    get_coll_type,
    create_coll_type,
    update_coll_type,
    delete_coll_type,
    "/lookups/coll-types",
    "/lookups/coll-types/{id}",
    "coll-types",
    CollType,
    UpsertCollType,
    "SELECT coll_type_id, coll_type_name FROM mst_coll_type WHERE coll_type_id = ?",
    "INSERT INTO mst_coll_type (coll_type_name, input_date, last_update) VALUES (?, CURDATE(), CURDATE())",
    "UPDATE mst_coll_type SET coll_type_name = ?, last_update = CURDATE() WHERE coll_type_id = ?",
    "DELETE FROM mst_coll_type WHERE coll_type_id = ?",
    [coll_type_name]
);

string_lookup_crud!(
    get_location,
    create_location,
    update_location,
    delete_location,
    "/lookups/locations",
    "/lookups/locations/{id}",
    "locations",
    Location,
    UpsertLocation,
    location_id,
    "SELECT location_id, location_name FROM mst_location WHERE location_id = ?",
    "INSERT INTO mst_location (location_id, location_name, input_date, last_update) VALUES (?, ?, CURDATE(), CURDATE())",
    "UPDATE mst_location SET location_id = ?, location_name = ?, last_update = CURDATE() WHERE location_id = ?",
    "DELETE FROM mst_location WHERE location_id = ?",
    [location_id, location_name],
    [location_id, location_name]
);

string_lookup_crud!(
    get_language,
    create_language,
    update_language,
    delete_language,
    "/lookups/languages",
    "/lookups/languages/{id}",
    "languages",
    Language,
    UpsertLanguage,
    language_id,
    "SELECT language_id, language_name FROM mst_language WHERE language_id = ?",
    "INSERT INTO mst_language (language_id, language_name, input_date, last_update) VALUES (?, ?, CURDATE(), CURDATE())",
    "UPDATE mst_language SET language_id = ?, language_name = ?, last_update = CURDATE() WHERE language_id = ?",
    "DELETE FROM mst_language WHERE language_id = ?",
    [language_id, language_name],
    [language_id, language_name]
);

numeric_lookup_crud!(
    get_gmd,
    create_gmd,
    update_gmd,
    delete_gmd,
    "/lookups/gmd",
    "/lookups/gmd/{id}",
    "gmd",
    Gmd,
    UpsertGmd,
    "SELECT gmd_id, gmd_code, gmd_name, icon_image FROM mst_gmd WHERE gmd_id = ?",
    "INSERT INTO mst_gmd (gmd_code, gmd_name, icon_image, input_date, last_update) VALUES (?, ?, ?, CURDATE(), CURDATE())",
    "UPDATE mst_gmd SET gmd_code = ?, gmd_name = ?, icon_image = ?, last_update = CURDATE() WHERE gmd_id = ?",
    "DELETE FROM mst_gmd WHERE gmd_id = ?",
    [gmd_code, gmd_name, icon_image]
);

string_lookup_crud!(
    get_item_status,
    create_item_status,
    update_item_status,
    delete_item_status,
    "/lookups/item-statuses",
    "/lookups/item-statuses/{id}",
    "item-statuses",
    ItemStatus,
    UpsertItemStatus,
    item_status_id,
    "SELECT item_status_id, item_status_name, no_loan, rules, skip_stock_take FROM mst_item_status WHERE item_status_id = ?",
    "INSERT INTO mst_item_status (item_status_id, item_status_name, no_loan, rules, skip_stock_take, input_date, last_update) VALUES (?, ?, ?, ?, ?, CURDATE(), CURDATE())",
    "UPDATE mst_item_status SET item_status_id = ?, item_status_name = ?, no_loan = ?, rules = ?, skip_stock_take = ?, last_update = CURDATE() WHERE item_status_id = ?",
    "DELETE FROM mst_item_status WHERE item_status_id = ?",
    [
        item_status_id,
        item_status_name,
        no_loan,
        rules,
        skip_stock_take
    ],
    [
        item_status_id,
        item_status_name,
        no_loan,
        rules,
        skip_stock_take
    ]
);

numeric_lookup_crud!(
    get_frequency,
    create_frequency,
    update_frequency,
    delete_frequency,
    "/lookups/frequencies",
    "/lookups/frequencies/{id}",
    "frequencies",
    Frequency,
    UpsertFrequency,
    "SELECT frequency_id, frequency, language_prefix, time_increment, time_unit FROM mst_frequency WHERE frequency_id = ?",
    "INSERT INTO mst_frequency (frequency, language_prefix, time_increment, time_unit, input_date, last_update) VALUES (?, ?, ?, ?, CURDATE(), CURDATE())",
    "UPDATE mst_frequency SET frequency = ?, language_prefix = ?, time_increment = ?, time_unit = ?, last_update = CURDATE() WHERE frequency_id = ?",
    "DELETE FROM mst_frequency WHERE frequency_id = ?",
    [frequency, language_prefix, time_increment, time_unit]
);

numeric_lookup_crud!(
    get_module,
    create_module,
    update_module,
    delete_module,
    "/lookups/modules",
    "/lookups/modules/{id}",
    "modules",
    Module,
    UpsertModule,
    "SELECT module_id, module_name, module_path, module_desc FROM mst_module WHERE module_id = ?",
    "INSERT INTO mst_module (module_name, module_path, module_desc) VALUES (?, ?, ?)",
    "UPDATE mst_module SET module_name = ?, module_path = ?, module_desc = ? WHERE module_id = ?",
    "DELETE FROM mst_module WHERE module_id = ?",
    [module_name, module_path, module_desc]
);

numeric_lookup_crud!(
    get_place,
    create_place,
    update_place,
    delete_place,
    "/lookups/places",
    "/lookups/places/{id}",
    "places",
    Place,
    UpsertPlace,
    "SELECT place_id, place_name FROM mst_place WHERE place_id = ?",
    "INSERT INTO mst_place (place_name, input_date, last_update) VALUES (?, CURDATE(), CURDATE())",
    "UPDATE mst_place SET place_name = ?, last_update = CURDATE() WHERE place_id = ?",
    "DELETE FROM mst_place WHERE place_id = ?",
    [place_name]
);

numeric_lookup_crud!(
    get_publisher,
    create_publisher,
    update_publisher,
    delete_publisher,
    "/lookups/publishers",
    "/lookups/publishers/{id}",
    "publishers",
    Publisher,
    UpsertPublisher,
    "SELECT publisher_id, publisher_name FROM mst_publisher WHERE publisher_id = ?",
    "INSERT INTO mst_publisher (publisher_name, input_date, last_update) VALUES (?, CURDATE(), CURDATE())",
    "UPDATE mst_publisher SET publisher_name = ?, last_update = CURDATE() WHERE publisher_id = ?",
    "DELETE FROM mst_publisher WHERE publisher_id = ?",
    [publisher_name]
);

numeric_lookup_crud!(
    get_supplier,
    create_supplier,
    update_supplier,
    delete_supplier,
    "/lookups/suppliers",
    "/lookups/suppliers/{id}",
    "suppliers",
    Supplier,
    UpsertSupplier,
    "SELECT supplier_id, supplier_name, address, postal_code, phone, contact, fax, account, e_mail FROM mst_supplier WHERE supplier_id = ?",
    "INSERT INTO mst_supplier (supplier_name, address, postal_code, phone, contact, fax, account, e_mail, input_date, last_update) VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURDATE(), CURDATE())",
    "UPDATE mst_supplier SET supplier_name = ?, address = ?, postal_code = ?, phone = ?, contact = ?, fax = ?, account = ?, e_mail = ?, last_update = CURDATE() WHERE supplier_id = ?",
    "DELETE FROM mst_supplier WHERE supplier_id = ?",
    [
        supplier_name,
        address,
        postal_code,
        phone,
        contact,
        fax,
        account,
        e_mail
    ]
);

async fn ensure_authority_type(state: &AppState, id: &str) -> Result<(), AppError> {
    let exists: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM mst_authority_type WHERE authority_type_id = ?")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;
    if exists == 0 {
        return Err(AppError::BadRequest(format!(
            "authority_type {id} tidak ditemukan"
        )));
    }
    Ok(())
}

#[utoipa::path(get, path = "/lookups/authors/{id}", params(("id" = String, Path, description = "Resource ID")), responses((status = 200, body = JsonApiDocument)), security(("bearerAuth" = [])), tag = "Lookups")]
async fn get_author(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;
    Ok(Json(lookup_by_id::<Author>(&state, "SELECT author_id, author_name, author_year, authority_type, auth_list FROM mst_author WHERE author_id = ?", &id, "authors").await?))
}

#[utoipa::path(post, path = "/lookups/authors", request_body = UpsertAuthor, responses((status = 200, body = JsonApiDocument)), security(("bearerAuth" = [])), tag = "Lookups")]
async fn create_author(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpsertAuthor>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Write)?;
    ensure_authority_type(&state, &payload.authority_type).await?;
    let result = sqlx::query("INSERT INTO mst_author (author_name, author_year, authority_type, auth_list, input_date, last_update) VALUES (?, ?, ?, ?, CURDATE(), CURDATE())")
        .bind(&payload.author_name)
        .bind(&payload.author_year)
        .bind(&payload.authority_type)
        .bind(&payload.auth_list)
        .execute(&state.pool)
        .await?;
    let id = result.last_insert_id().to_string();
    Ok(Json(lookup_by_id::<Author>(&state, "SELECT author_id, author_name, author_year, authority_type, auth_list FROM mst_author WHERE author_id = ?", &id, "authors").await?))
}

#[utoipa::path(put, path = "/lookups/authors/{id}", request_body = UpsertAuthor, params(("id" = String, Path, description = "Resource ID")), responses((status = 200, body = JsonApiDocument)), security(("bearerAuth" = [])), tag = "Lookups")]
async fn update_author(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpsertAuthor>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Write)?;
    ensure_authority_type(&state, &payload.authority_type).await?;
    lookup_by_id::<Author>(&state, "SELECT author_id, author_name, author_year, authority_type, auth_list FROM mst_author WHERE author_id = ?", &id, "authors").await?;
    sqlx::query("UPDATE mst_author SET author_name = ?, author_year = ?, authority_type = ?, auth_list = ?, last_update = CURDATE() WHERE author_id = ?")
        .bind(&payload.author_name)
        .bind(&payload.author_year)
        .bind(&payload.authority_type)
        .bind(&payload.auth_list)
        .bind(&id)
        .execute(&state.pool)
        .await?;
    Ok(Json(lookup_by_id::<Author>(&state, "SELECT author_id, author_name, author_year, authority_type, auth_list FROM mst_author WHERE author_id = ?", &id, "authors").await?))
}

#[utoipa::path(delete, path = "/lookups/authors/{id}", params(("id" = String, Path, description = "Resource ID")), responses((status = 204, description = "Resource deleted")), security(("bearerAuth" = [])), tag = "Lookups")]
async fn delete_author(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Write)?;
    remove_lookup(&state, "DELETE FROM mst_author WHERE author_id = ?", &id).await
}

string_lookup_crud!(
    get_authority_type,
    create_authority_type,
    update_authority_type,
    delete_authority_type,
    "/lookups/authority-types",
    "/lookups/authority-types/{id}",
    "authority-types",
    AuthorityType,
    UpsertAuthorityType,
    authority_type_id,
    "SELECT authority_type_id, authority_type_name FROM mst_authority_type WHERE authority_type_id = ?",
    "INSERT INTO mst_authority_type (authority_type_id, authority_type_name, input_date, last_update) VALUES (?, ?, CURDATE(), CURDATE())",
    "UPDATE mst_authority_type SET authority_type_id = ?, authority_type_name = ?, last_update = CURDATE() WHERE authority_type_id = ?",
    "DELETE FROM mst_authority_type WHERE authority_type_id = ?",
    [authority_type_id, authority_type_name],
    [authority_type_id, authority_type_name]
);

numeric_lookup_crud!(
    get_authority_level,
    create_authority_level,
    update_authority_level,
    delete_authority_level,
    "/lookups/authority-levels",
    "/lookups/authority-levels/{id}",
    "authority-levels",
    AuthorityLevel,
    UpsertAuthorityLevel,
    "SELECT authority_level_id, authority_level_name FROM mst_authority_level WHERE authority_level_id = ?",
    "INSERT INTO mst_authority_level (authority_level_name, input_date, last_update) VALUES (?, CURDATE(), CURDATE())",
    "UPDATE mst_authority_level SET authority_level_name = ?, last_update = CURDATE() WHERE authority_level_id = ?",
    "DELETE FROM mst_authority_level WHERE authority_level_id = ?",
    [authority_level_name]
);

numeric_lookup_crud!(
    get_topic,
    create_topic,
    update_topic,
    delete_topic,
    "/lookups/topics",
    "/lookups/topics/{id}",
    "topics",
    Topic,
    UpsertTopic,
    "SELECT topic_id, topic, topic_type, auth_list, classification FROM mst_topic WHERE topic_id = ?",
    "INSERT INTO mst_topic (topic, topic_type, auth_list, classification, input_date, last_update) VALUES (?, ?, ?, ?, CURDATE(), CURDATE())",
    "UPDATE mst_topic SET topic = ?, topic_type = ?, auth_list = ?, classification = ?, last_update = CURDATE() WHERE topic_id = ?",
    "DELETE FROM mst_topic WHERE topic_id = ?",
    [topic, topic_type, auth_list, classification]
);

numeric_lookup_crud!(
    get_content_type,
    create_content_type,
    update_content_type,
    delete_content_type,
    "/lookups/content-types",
    "/lookups/content-types/{id}",
    "content-types",
    ContentType,
    UpsertContentType,
    "SELECT id, content_type, code, code2 FROM mst_content_type WHERE id = ?",
    "INSERT INTO mst_content_type (content_type, code, code2, input_date, last_update) VALUES (?, ?, ?, NOW(), NOW())",
    "UPDATE mst_content_type SET content_type = ?, code = ?, code2 = ?, last_update = NOW() WHERE id = ?",
    "DELETE FROM mst_content_type WHERE id = ?",
    [content_type, code, code2]
);

numeric_lookup_crud!(
    get_media_type,
    create_media_type,
    update_media_type,
    delete_media_type,
    "/lookups/media-types",
    "/lookups/media-types/{id}",
    "media-types",
    MediaType,
    UpsertMediaType,
    "SELECT id, media_type, code, code2 FROM mst_media_type WHERE id = ?",
    "INSERT INTO mst_media_type (media_type, code, code2, input_date, last_update) VALUES (?, ?, ?, NOW(), NOW())",
    "UPDATE mst_media_type SET media_type = ?, code = ?, code2 = ?, last_update = NOW() WHERE id = ?",
    "DELETE FROM mst_media_type WHERE id = ?",
    [media_type, code, code2]
);

numeric_lookup_crud!(
    get_carrier_type,
    create_carrier_type,
    update_carrier_type,
    delete_carrier_type,
    "/lookups/carrier-types",
    "/lookups/carrier-types/{id}",
    "carrier-types",
    CarrierType,
    UpsertCarrierType,
    "SELECT id, carrier_type, code, code2 FROM mst_carrier_type WHERE id = ?",
    "INSERT INTO mst_carrier_type (carrier_type, code, code2, input_date, last_update) VALUES (?, ?, ?, NOW(), NOW())",
    "UPDATE mst_carrier_type SET carrier_type = ?, code = ?, code2 = ?, last_update = NOW() WHERE id = ?",
    "DELETE FROM mst_carrier_type WHERE id = ?",
    [carrier_type, code, code2]
);

string_lookup_crud!(
    get_relation_term,
    create_relation_term,
    update_relation_term,
    delete_relation_term,
    "/lookups/relation-terms",
    "/lookups/relation-terms/{id}",
    "relation-terms",
    RelationTerm,
    UpsertRelationTerm,
    rt_id,
    "SELECT rt_id, rt_desc FROM mst_relation_term WHERE rt_id = ?",
    "INSERT INTO mst_relation_term (rt_id, rt_desc) VALUES (?, ?)",
    "UPDATE mst_relation_term SET rt_id = ?, rt_desc = ? WHERE rt_id = ?",
    "DELETE FROM mst_relation_term WHERE rt_id = ?",
    [rt_id, rt_desc],
    [rt_id, rt_desc]
);

numeric_lookup_crud!(
    get_loan_rule,
    create_loan_rule,
    update_loan_rule,
    delete_loan_rule,
    "/lookups/loan-rules",
    "/lookups/loan-rules/{id}",
    "loan-rules",
    LoanRule,
    UpsertLoanRule,
    "SELECT loan_rules_id, member_type_id, coll_type_id, gmd_id, loan_limit, loan_periode, reborrow_limit, fine_each_day, grace_periode FROM mst_loan_rules WHERE loan_rules_id = ?",
    "INSERT INTO mst_loan_rules (member_type_id, coll_type_id, gmd_id, loan_limit, loan_periode, reborrow_limit, fine_each_day, grace_periode, input_date, last_update) VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURDATE(), CURDATE())",
    "UPDATE mst_loan_rules SET member_type_id = ?, coll_type_id = ?, gmd_id = ?, loan_limit = ?, loan_periode = ?, reborrow_limit = ?, fine_each_day = ?, grace_periode = ?, last_update = CURDATE() WHERE loan_rules_id = ?",
    "DELETE FROM mst_loan_rules WHERE loan_rules_id = ?",
    [
        member_type_id,
        coll_type_id,
        gmd_id,
        loan_limit,
        loan_periode,
        reborrow_limit,
        fine_each_day,
        grace_periode
    ]
);

#[utoipa::path(
    get,
    path = "/lookups/member-types",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn member_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = paged_lookup(
        &state,
        pagination,
        "SELECT member_type_id, member_type_name, loan_limit, loan_periode, enable_reserve, reserve_limit, member_periode, reborrow_limit, fine_each_day, grace_periode FROM mst_member_type ORDER BY member_type_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_member_type",
        "member-types",
        |row: &MemberType| row.member_type_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/coll-types",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn coll_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = paged_lookup(
        &state,
        pagination,
        "SELECT coll_type_id, coll_type_name FROM mst_coll_type ORDER BY coll_type_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_coll_type",
        "coll-types",
        |row: &CollType| row.coll_type_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/locations",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn locations(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = paged_lookup(
        &state,
        pagination,
        "SELECT location_id, location_name FROM mst_location ORDER BY location_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_location",
        "locations",
        |row: &Location| row.location_id.clone(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/languages",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn languages(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT language_id, language_name FROM mst_language ORDER BY language_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_language",
        "language_id, language_name",
        "languages",
        |row: &Language| row.language_id.clone(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/gmd",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn gmds(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT gmd_id, gmd_code, gmd_name, icon_image FROM mst_gmd ORDER BY gmd_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_gmd",
        "gmd_code, gmd_name, icon_image",
        "gmd",
        |row: &Gmd| row.gmd_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/item-statuses",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn item_statuses(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = paged_lookup(
        &state,
        pagination,
        "SELECT item_status_id, item_status_name, no_loan, rules, skip_stock_take FROM mst_item_status ORDER BY item_status_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_item_status",
        "item-statuses",
        |row: &ItemStatus| row.item_status_id.clone(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/frequencies",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn frequencies(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT frequency_id, frequency, language_prefix, time_increment, time_unit FROM mst_frequency ORDER BY frequency_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_frequency",
        "frequency, language_prefix, time_increment, time_unit",
        "frequencies",
        |row: &Frequency| row.frequency_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/modules",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn modules(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = paged_lookup(
        &state,
        pagination,
        "SELECT module_id, module_name, module_path, module_desc FROM mst_module ORDER BY module_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_module",
        "modules",
        |row: &Module| row.module_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/places",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn places(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT place_id, place_name FROM mst_place ORDER BY place_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_place",
        "place_name",
        "places",
        |row: &Place| row.place_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/publishers",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn publishers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT publisher_id, publisher_name FROM mst_publisher ORDER BY publisher_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_publisher",
        "publisher_name",
        "publishers",
        |row: &Publisher| row.publisher_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/suppliers",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn suppliers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = paged_lookup(
        &state,
        pagination,
        "SELECT supplier_id, supplier_name, address, postal_code, phone, contact, fax, account, e_mail FROM mst_supplier ORDER BY supplier_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_supplier",
        "suppliers",
        |row: &Supplier| row.supplier_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/authors",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn authors(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT author_id, author_name, author_year, authority_type, auth_list FROM mst_author ORDER BY author_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_author",
        "author_name, author_year, authority_type, auth_list",
        "authors",
        |row: &Author| row.author_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/authority-types",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn authority_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;
    Ok(Json(
        paged_lookup(
            &state,
            pagination,
            "SELECT authority_type_id, authority_type_name FROM mst_authority_type ORDER BY authority_type_id LIMIT ? OFFSET ?",
            "SELECT COUNT(*) FROM mst_authority_type",
            "authority-types",
            |row: &AuthorityType| row.authority_type_id.clone(),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/lookups/authority-levels",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn authority_levels(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;
    Ok(Json(
        paged_lookup(
            &state,
            pagination,
            "SELECT authority_level_id, authority_level_name FROM mst_authority_level ORDER BY authority_level_id LIMIT ? OFFSET ?",
            "SELECT COUNT(*) FROM mst_authority_level",
            "authority-levels",
            |row: &AuthorityLevel| row.authority_level_id.to_string(),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/lookups/topics",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn topics(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT topic_id, topic, topic_type, auth_list, classification FROM mst_topic ORDER BY topic_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_topic",
        "topic, topic_type, auth_list, classification",
        "topics",
        |row: &Topic| row.topic_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/content-types",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn content_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT id, content_type, code, code2 FROM mst_content_type ORDER BY id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_content_type",
        "content_type, code, code2",
        "content-types",
        |row: &ContentType| row.id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/media-types",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn media_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT id, media_type, code, code2 FROM mst_media_type ORDER BY id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_media_type",
        "media_type, code, code2",
        "media-types",
        |row: &MediaType| row.id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/carrier-types",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn carrier_types(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT id, carrier_type, code, code2 FROM mst_carrier_type ORDER BY id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_carrier_type",
        "carrier_type, code, code2",
        "carrier-types",
        |row: &CarrierType| row.id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/relation-terms",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn relation_terms(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LookupParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = searchable_paged_lookup(
        &state,
        params,
        "SELECT rt_id, rt_desc FROM mst_relation_term ORDER BY rt_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_relation_term",
        "rt_id, rt_desc",
        "relation-terms",
        |row: &RelationTerm| row.rt_id.clone(),
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    get,
    path = "/lookups/loan-rules",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Lookups"
)]
async fn loan_rules(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<Pagination>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::MasterFile, Permission::Read)?;

    let document = paged_lookup(
        &state,
        pagination,
        "SELECT loan_rules_id, member_type_id, coll_type_id, gmd_id, loan_limit, loan_periode, reborrow_limit, fine_each_day, grace_periode FROM mst_loan_rules ORDER BY loan_rules_id LIMIT ? OFFSET ?",
        "SELECT COUNT(*) FROM mst_loan_rules",
        "loan-rules",
        |row: &LoanRule| row.loan_rules_id.to_string(),
    )
    .await?;

    Ok(Json(document))
}

#[cfg(test)]
mod tests {
    use super::LookupParams;
    use axum::{extract::Query, http::Uri};

    #[test]
    fn lookup_params_accept_search_and_json_api_pagination() {
        let uri: Uri = "/?page%5Bnumber%5D=2&page%5Bsize%5D=25&q=audio%20disc"
            .parse()
            .unwrap();
        let Query(params) = Query::<LookupParams>::try_from_uri(&uri).unwrap();
        let (_, _, page, per_page) = params.pagination.limit_offset();

        assert_eq!((page, per_page), (2, 25));
        assert_eq!(params.q.as_deref(), Some("audio disc"));
    }
}
