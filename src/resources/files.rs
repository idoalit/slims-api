use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    auth::{AuthUser, Role},
    config::AppState,
    error::AppError,
    resources::{ListParams, PagedResponse},
};

#[derive(Debug, Serialize, Deserialize, FromRow)]
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
    pub input_date: String,
    pub last_update: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct FileBiblioAttachment {
    pub biblio_id: i64,
    pub title: String,
    pub placement: Option<String>,
    pub access_type: String,
    pub access_limit: Option<String>,
}

#[derive(Debug, Serialize)]
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

async fn list_files(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<PagedResponse<FileResponse>>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff])?;

    let pagination = params.pagination();
    let includes = params.includes();
    let (limit, offset, page, per_page) = pagination.limit_offset();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
        .fetch_one(&state.pool)
        .await?;

    let files = sqlx::query_as::<_, FileObject>(
        "SELECT file_id, file_title, file_name, file_url, file_dir, mime_type, file_desc, file_key, uploader_id, input_date, last_update FROM files ORDER BY file_id DESC LIMIT ? OFFSET ?",
    )
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

    Ok(Json(PagedResponse {
        data,
        page,
        per_page,
        total,
    }))
}

async fn get_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
    Path(file_id): Path<i64>,
) -> Result<Json<FileResponse>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff])?;

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

    Ok(Json(FileResponse { file, biblios }))
}
