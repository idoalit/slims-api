use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, State},
    routing::post,
};
use rand::RngCore;
use serde::Serialize;
use std::path::Path;
use utoipa::ToSchema;

use crate::{
    auth::{AuthUser, ModuleAccess, Permission},
    config::AppState,
    error::AppError,
    jsonapi::{JsonApiDocument, resource, single_document},
};

const MAX_COVER_BYTES: usize = 5 * 1024 * 1024;
const MAX_ATTACHMENT_BYTES: usize = 128 * 1024 * 1024;
const MULTIPART_OVERHEAD_BYTES: usize = 512 * 1024;

#[derive(Debug, ToSchema)]
#[allow(dead_code)]
pub struct UploadCoverForm {
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

#[derive(Debug, ToSchema)]
#[allow(dead_code)]
pub struct UploadAttachmentForm {
    #[schema(value_type = String, format = Binary)]
    pub file: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponse {
    pub key: String,
    pub url: String,
    pub file_name: String,
    pub content_type: String,
    pub size: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadAttachmentResponse {
    pub file_id: i64,
    pub key: String,
    pub url: String,
    pub file_title: String,
    pub file_name: String,
    pub content_type: String,
    pub description: Option<String>,
    pub size: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/bibliography-covers", post(upload_bibliography_cover))
        .route(
            "/bibliography-attachments",
            post(upload_bibliography_attachment),
        )
        .layer(DefaultBodyLimit::max(
            MAX_ATTACHMENT_BYTES + MULTIPART_OVERHEAD_BYTES,
        ))
}

#[utoipa::path(
    post,
    path = "/uploads/bibliography-attachments",
    request_body(content = UploadAttachmentForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Lampiran berhasil diunggah", body = JsonApiDocument),
        (status = 400, description = "Berkas tidak valid"),
        (status = 503, description = "Penyimpanan file dinonaktifkan")
    ),
    security(("bearerAuth" = [])),
    tag = "Uploads"
)]
pub async fn upload_bibliography_attachment(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Write)?;
    let storage = state
        .file_storage
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("penyimpanan file dinonaktifkan".into()))?;

    let mut title = None;
    let mut description = None;
    let mut upload = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| AppError::BadRequest(format!("multipart tidak valid: {error}")))?
    {
        match field.name() {
            Some("title") => {
                title = optional_multipart_text(field, "title", 255).await?;
            }
            Some("description") => {
                description = optional_multipart_text(field, "description", 65_535).await?;
            }
            Some("file") => {
                if upload.is_some() {
                    return Err(AppError::BadRequest(
                        "hanya satu lampiran per permintaan upload".into(),
                    ));
                }
                let file_name = field
                    .file_name()
                    .unwrap_or("attachment")
                    .chars()
                    .take(255)
                    .collect::<String>();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_ascii_lowercase();
                if is_dangerous_attachment_type(&content_type) {
                    return Err(AppError::BadRequest(
                        "tipe berkas aktif seperti HTML, SVG, JavaScript, atau executable tidak diizinkan"
                            .into(),
                    ));
                }
                let bytes = field.bytes().await.map_err(|error| {
                    AppError::BadRequest(format!("gagal membaca berkas: {error}"))
                })?;
                if bytes.is_empty() {
                    return Err(AppError::BadRequest("berkas lampiran kosong".into()));
                }
                if bytes.len() > MAX_ATTACHMENT_BYTES {
                    return Err(AppError::BadRequest(
                        "ukuran lampiran maksimal 128 MB".into(),
                    ));
                }
                upload = Some((file_name, content_type, bytes.to_vec()));
            }
            _ => {}
        }
    }

    let (file_name, content_type, bytes) =
        upload.ok_or_else(|| AppError::BadRequest("field file wajib diisi".into()))?;
    let file_title = title.unwrap_or_else(|| file_stem(&file_name));
    let extension = safe_file_extension(&file_name);
    let object_name = match extension {
        Some(extension) => format!("attachments/{}.{}", random_hex_id(), extension),
        None => format!("attachments/{}", random_hex_id()),
    };
    let size = bytes.len();
    let (key, url) = storage
        .put(&object_name, &content_type, bytes)
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, "attachment upload failed");
            AppError::Internal("gagal mengunggah lampiran".into())
        })?;
    let file_dir = key.rsplit_once('/').map(|(directory, _)| directory);
    let uploader_id = auth
        .claims
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::Internal("ID pengguna pada token tidak valid".into()))?;
    let result = sqlx::query(
        "INSERT INTO files (file_title, file_name, file_url, file_dir, mime_type, file_desc, file_key, uploader_id, input_date, last_update) VALUES (?, ?, ?, ?, ?, ?, NULL, ?, NOW(), NOW())",
    )
    .bind(&file_title)
    .bind(&file_name)
    .bind(&url)
    .bind(file_dir)
    .bind(&content_type)
    .bind(&description)
    .bind(uploader_id)
    .execute(&state.pool)
    .await?;
    let file_id = result.last_insert_id() as i64;

    let response = UploadAttachmentResponse {
        file_id,
        key,
        url,
        file_title,
        file_name,
        content_type,
        description,
        size,
    };
    Ok(Json(single_document(resource(
        "uploads",
        file_id.to_string(),
        response,
    ))))
}

async fn optional_multipart_text(
    field: axum::extract::multipart::Field<'_>,
    name: &str,
    max_length: usize,
) -> Result<Option<String>, AppError> {
    let value = field
        .text()
        .await
        .map_err(|error| AppError::BadRequest(format!("field {name} tidak valid: {error}")))?;
    let value = value.trim();
    if value.len() > max_length {
        return Err(AppError::BadRequest(format!(
            "field {name} maksimal {max_length} karakter"
        )));
    }
    Ok((!value.is_empty()).then(|| value.to_string()))
}

fn safe_file_extension(file_name: &str) -> Option<String> {
    Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 12
                && value
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
                && !matches!(
                    value.to_ascii_lowercase().as_str(),
                    "html"
                        | "htm"
                        | "svg"
                        | "js"
                        | "mjs"
                        | "cjs"
                        | "exe"
                        | "dll"
                        | "bat"
                        | "cmd"
                        | "com"
                        | "sh"
                        | "php"
                )
        })
        .map(|value| value.to_ascii_lowercase())
}

fn file_stem(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Lampiran")
        .chars()
        .take(255)
        .collect()
}

fn is_dangerous_attachment_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "text/html"
            | "image/svg+xml"
            | "application/javascript"
            | "text/javascript"
            | "application/x-msdownload"
            | "application/x-executable"
            | "application/x-sh"
    )
}

#[utoipa::path(
    post,
    path = "/uploads/bibliography-covers",
    request_body(content = UploadCoverForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Sampul berhasil diunggah", body = JsonApiDocument),
        (status = 400, description = "Berkas tidak valid"),
        (status = 503, description = "Penyimpanan file dinonaktifkan")
    ),
    security(("bearerAuth" = [])),
    tag = "Uploads"
)]
pub async fn upload_bibliography_cover(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Bibliography, Permission::Write)?;
    let storage = state
        .file_storage
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("penyimpanan file dinonaktifkan".into()))?;

    let mut upload = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| AppError::BadRequest(format!("multipart tidak valid: {error}")))?
    {
        if field.name() != Some("file") {
            continue;
        }
        if upload.is_some() {
            return Err(AppError::BadRequest(
                "hanya satu berkas sampul yang dapat diunggah".into(),
            ));
        }

        let file_name = field
            .file_name()
            .unwrap_or("cover")
            .chars()
            .take(255)
            .collect::<String>();
        let content_type = field
            .content_type()
            .ok_or_else(|| AppError::BadRequest("tipe berkas sampul wajib tersedia".into()))?
            .to_ascii_lowercase();
        let extension = extension_for_content_type(&content_type).ok_or_else(|| {
            AppError::BadRequest("sampul harus berupa JPEG, PNG, WebP, atau GIF".into())
        })?;
        let bytes = field
            .bytes()
            .await
            .map_err(|error| AppError::BadRequest(format!("gagal membaca berkas: {error}")))?;
        if bytes.is_empty() {
            return Err(AppError::BadRequest("berkas sampul kosong".into()));
        }
        if bytes.len() > MAX_COVER_BYTES {
            return Err(AppError::BadRequest("ukuran sampul maksimal 5 MB".into()));
        }
        if !has_expected_signature(&content_type, &bytes) {
            return Err(AppError::BadRequest(
                "isi berkas tidak sesuai dengan tipe gambar".into(),
            ));
        }
        upload = Some((file_name, content_type, extension, bytes.to_vec()));
    }

    let (file_name, content_type, extension, bytes) =
        upload.ok_or_else(|| AppError::BadRequest("field file wajib diisi".into()))?;
    let size = bytes.len();
    let key_suffix = format!("{}.{}", random_hex_id(), extension);
    let (key, url) = storage
        .put(&key_suffix, &content_type, bytes)
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, "cover upload failed");
            AppError::Internal("gagal mengunggah sampul".into())
        })?;
    let response = UploadResponse {
        key: key.clone(),
        url,
        file_name,
        content_type,
        size,
    };

    Ok(Json(single_document(resource("uploads", key, response))))
}

fn extension_for_content_type(content_type: &str) -> Option<&'static str> {
    match content_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        _ => None,
    }
}

fn has_expected_signature(content_type: &str, bytes: &[u8]) -> bool {
    match content_type {
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/png" => bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        _ => false,
    }
}

fn random_hex_id() -> String {
    let mut bytes = [0_u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restricts_cover_content_types() {
        assert_eq!(extension_for_content_type("image/jpeg"), Some("jpg"));
        assert_eq!(extension_for_content_type("image/webp"), Some("webp"));
        assert_eq!(extension_for_content_type("image/svg+xml"), None);
    }

    #[test]
    fn verifies_file_signatures_instead_of_trusting_mime_only() {
        assert!(has_expected_signature(
            "image/png",
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        ));
        assert!(!has_expected_signature("image/png", b"not an image"));
    }

    #[test]
    fn generates_unpredictable_object_names() {
        let first = random_hex_id();
        let second = random_hex_id();
        assert_eq!(first.len(), 32);
        assert_ne!(first, second);
    }

    #[test]
    fn keeps_document_extensions_but_drops_active_content_extensions() {
        assert_eq!(safe_file_extension("panduan.PDF"), Some("pdf".into()));
        assert_eq!(safe_file_extension("rekaman.mp4"), Some("mp4".into()));
        assert_eq!(safe_file_extension("halaman.html"), None);
        assert_eq!(safe_file_extension("script.js"), None);
    }

    #[test]
    fn rejects_active_content_mime_types() {
        assert!(is_dangerous_attachment_type("text/html"));
        assert!(is_dangerous_attachment_type("image/svg+xml"));
        assert!(!is_dangerous_attachment_type("application/pdf"));
        assert!(!is_dangerous_attachment_type("video/mp4"));
    }
}
