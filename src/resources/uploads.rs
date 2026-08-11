use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, State},
    routing::post,
};
use rand::RngCore;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    auth::{AuthUser, ModuleAccess, Permission},
    config::AppState,
    error::AppError,
    jsonapi::{JsonApiDocument, resource, single_document},
};

const MAX_COVER_BYTES: usize = 5 * 1024 * 1024;
const MULTIPART_OVERHEAD_BYTES: usize = 512 * 1024;

#[derive(Debug, ToSchema)]
#[allow(dead_code)]
pub struct UploadCoverForm {
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponse {
    pub key: String,
    pub url: String,
    pub file_name: String,
    pub content_type: String,
    pub size: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/bibliography-covers", post(upload_bibliography_cover))
        .layer(DefaultBodyLimit::max(
            MAX_COVER_BYTES + MULTIPART_OVERHEAD_BYTES,
        ))
}

#[utoipa::path(
    post,
    path = "/uploads/bibliography-covers",
    request_body(content = UploadCoverForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Sampul berhasil diunggah", body = JsonApiDocument),
        (status = 400, description = "Berkas tidak valid"),
        (status = 503, description = "Object storage belum dikonfigurasi")
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
        .object_storage
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("object storage belum dikonfigurasi".into()))?;

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
}
