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
    resources::{
        FilterField, FilterOperator, FilterValueType, ListParams, bind_filters_to_query,
        bind_filters_to_scalar, where_clause,
    },
};

const FILE_FILTERS: &[FilterField<'static>] = &[FilterField::new(
    "search",
    "CONCAT_WS(' ', file_title, file_name, mime_type)",
    FilterOperator::Like,
    FilterValueType::Text,
)];

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FileObject {
    pub file_id: i64,
    pub file_title: String,
    pub file_name: String,
    pub file_url: Option<String>,
    pub file_dir: Option<String>,
    pub mime_type: Option<String>,
    pub file_desc: Option<String>,
    pub file_key: Option<String>,
    pub uploader_id: i64,
    pub input_date: NaiveDateTime,
    pub last_update: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FileBiblioAttachment {
    pub biblio_id: i64,
    pub title: String,
    pub placement: Option<String>,
    pub access_type: String,
    pub access_limit: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FileResponse {
    #[serde(flatten)]
    pub file: FileObject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biblios: Option<Vec<FileBiblioAttachment>>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_files))
        .route("/:file_id", get(get_file))
}

#[utoipa::path(
    get,
    path = "/files",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Files"
)]
async fn list_files(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Read)?;

    let pagination = params.pagination();
    let includes = params.includes();
    let file_fields = params.fieldset("files");
    let filters = params.filter_clauses(FILE_FILTERS)?;
    let where_sql = where_clause(&filters);
    let (limit, offset, page, per_page) = pagination.limit_offset();

    let count_sql = format!("SELECT COUNT(*) FROM files {where_sql}");
    let total: i64 = bind_filters_to_scalar(sqlx::query_scalar(&count_sql), &filters)
        .fetch_one(&state.pool)
        .await?;

    let data_sql = format!(
        "SELECT file_id, file_title, file_name, file_url, file_dir, mime_type, file_desc, file_key, uploader_id, input_date, last_update FROM files {where_sql} ORDER BY file_id DESC LIMIT ? OFFSET ?"
    );
    let files = bind_filters_to_query(sqlx::query_as::<_, FileObject>(&data_sql), &filters)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let mut data = Vec::with_capacity(files.len());

    for file in files {
        let biblios = if includes.contains("biblios") {
            let rows = sqlx::query_as::<_, FileBiblioAttachment>(
                "SELECT ba.biblio_id, b.title, ba.placement, ba.access_type, ba.access_limit FROM biblio_attachment ba JOIN biblio b ON b.biblio_id = ba.biblio_id WHERE ba.file_id = ?",
            )
            .bind(file.file_id)
            .fetch_all(&state.pool)
            .await?;
            Some(rows)
        } else {
            None
        };

        data.push(FileResponse { file, biblios });
    }

    let documents = data
        .into_iter()
        .map(|file| resource_with_fields("files", file.file.file_id.to_string(), file, file_fields))
        .collect();

    Ok(Json(collection_document(
        documents,
        pagination_meta(page, per_page, total),
    )))
}

#[utoipa::path(
    get,
    path = "/files/{file_id}",
    params(("file_id" = i64, Path, description = "File ID")),
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Files"
)]
async fn get_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
    Path(file_id): Path<i64>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Read)?;

    let file = sqlx::query_as::<_, FileObject>(
        "SELECT file_id, file_title, file_name, file_url, file_dir, mime_type, file_desc, file_key, uploader_id, input_date, last_update FROM files WHERE file_id = ?",
    )
    .bind(file_id)
    .fetch_one(&state.pool)
    .await?;

    let includes = params.includes();
    let biblios = if includes.contains("biblios") {
        let rows = sqlx::query_as::<_, FileBiblioAttachment>(
            "SELECT ba.biblio_id, b.title, ba.placement, ba.access_type, ba.access_limit FROM biblio_attachment ba JOIN biblio b ON b.biblio_id = ba.biblio_id WHERE ba.file_id = ?",
        )
        .bind(file.file_id)
        .fetch_all(&state.pool)
        .await?;
        Some(rows)
    } else {
        None
    };

    let response = FileResponse { file, biblios };
    let file_fields = params.fieldset("files");
    Ok(Json(single_document(resource_with_fields(
        "files",
        response.file.file_id.to_string(),
        response,
        file_fields,
    ))))
}
