use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
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
    resources::ListParams,
};

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SettingRow {
    pub setting_id: i64,
    pub setting_name: String,
    pub setting_value: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SettingResponse {
    pub setting_name: String,
    pub raw_value: Option<String>,
    #[schema(value_type = Object)]
    pub parsed_value: Option<JsonValue>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_settings))
        .route("/:setting_name", get(get_setting))
}

#[utoipa::path(
    get,
    path = "/settings",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Settings"
)]
async fn list_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::System, Permission::Read)?;

    let pagination = params.pagination();
    let (limit, offset, page, per_page) = pagination.limit_offset();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM setting")
        .fetch_one(&state.pool)
        .await?;

    let rows = sqlx::query_as::<_, SettingRow>(
        "SELECT setting_id, setting_name, setting_value FROM setting ORDER BY setting_name LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let setting_fields = params.fieldset("settings");
    let documents = rows
        .into_iter()
        .map(to_setting_response)
        .map(|setting| {
            let name = setting.setting_name.clone();
            resource_with_fields("settings", name, setting, setting_fields)
        })
        .collect();

    Ok(Json(collection_document(
        documents,
        pagination_meta(page, per_page, total),
    )))
}

#[utoipa::path(
    get,
    path = "/settings/{setting_name}",
    params(("setting_name" = String, Path, description = "Setting key; use dot notation for nested paths")),
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Settings"
)]
async fn get_setting(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
    Path(setting_path): Path<String>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::System, Permission::Read)?;

    let mut parts = setting_path.split('.').collect::<Vec<_>>();
    let base_name = parts.remove(0).to_string();

    let row = sqlx::query_as::<_, SettingRow>(
        "SELECT setting_id, setting_name, setting_value FROM setting WHERE setting_name = ?",
    )
    .bind(&base_name)
    .fetch_one(&state.pool)
    .await?;

    let mut resp = to_setting_response(row);

    if !parts.is_empty() {
        if let Some(parsed) = resp.parsed_value.clone() {
            if let Some(val) = extract_path(&parsed, &parts) {
                resp.parsed_value = Some(val);
            } else {
                resp.parsed_value = None;
            }
        } else {
            resp.parsed_value = None;
        }
        resp.setting_name = setting_path;
    }

    let id = resp.setting_name.clone();
    let setting_fields = params.fieldset("settings");
    Ok(Json(single_document(resource_with_fields(
        "settings",
        id,
        resp,
        setting_fields,
    ))))
}

fn to_setting_response(row: SettingRow) -> SettingResponse {
    let parsed_value = row
        .setting_value
        .as_ref()
        .and_then(|raw| parse_serialized_value(raw).ok());

    SettingResponse {
        setting_name: row.setting_name,
        raw_value: row.setting_value,
        parsed_value,
    }
}

fn parse_serialized_value(raw: &str) -> Result<JsonValue, AppError> {
    match unserialize(raw) {
        Ok(v) => Ok(v),
        Err(_) => Ok(JsonValue::String(raw.to_string())),
    }
}

fn extract_path(value: &JsonValue, path: &[&str]) -> Option<JsonValue> {
    let mut current = value;
    for segment in path {
        match current {
            JsonValue::Object(map) => {
                current = map.get(*segment)?;
            }
            JsonValue::Array(arr) => {
                let idx: usize = segment.parse().ok()?;
                current = arr.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(current.clone())
}

#[derive(Debug)]
enum SerializedTok<'a> {
    Str(&'a str),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Array(Vec<(SerializedTok<'a>, SerializedTok<'a>)>),
}

fn unserialize(input: &str) -> Result<JsonValue, String> {
    let bytes = input.as_bytes();
    let mut idx = 0;
    fn next_char(bytes: &[u8], idx: &mut usize) -> Option<u8> {
        if *idx >= bytes.len() {
            None
        } else {
            let c = bytes[*idx];
            *idx += 1;
            Some(c)
        }
    }

    fn parse_value<'a>(bytes: &'a [u8], idx: &mut usize) -> Result<SerializedTok<'a>, String> {
        match next_char(bytes, idx) {
            Some(b's') => {
                if next_char(bytes, idx) != Some(b':') {
                    return Err("expected :".into());
                }
                let len_start = *idx;
                while let Some(c) = next_char(bytes, idx) {
                    if c == b':' {
                        break;
                    }
                }
                let len_str =
                    std::str::from_utf8(&bytes[len_start..*idx - 1]).map_err(|_| "invalid utf8")?;
                let len: usize = len_str.parse().map_err(|_| "invalid length")?;
                // opening quote may be escaped in dump (\"), accept optional backslash
                match next_char(bytes, idx) {
                    Some(b'"') => {}
                    Some(b'\\') => {
                        if next_char(bytes, idx) != Some(b'"') {
                            return Err("expected opening quote after backslash".into());
                        }
                    }
                    _ => return Err("expected opening quote".into()),
                }
                let start = *idx;
                let end = start + len;
                if end > bytes.len() {
                    return Err("string out of bounds".into());
                }
                let s = std::str::from_utf8(&bytes[start..end]).map_err(|_| "invalid utf8")?;
                *idx = end;
                match next_char(bytes, idx) {
                    Some(b'"') => {}
                    Some(b'\\') => {
                        if next_char(bytes, idx) != Some(b'"') {
                            return Err("expected closing quote after backslash".into());
                        }
                    }
                    _ => return Err("expected closing quote".into()),
                }
                if next_char(bytes, idx) != Some(b';') {
                    return Err("expected ;".into());
                }
                Ok(SerializedTok::Str(s))
            }
            Some(b'i') => {
                if next_char(bytes, idx) != Some(b':') {
                    return Err("expected :".into());
                }
                let start = *idx;
                while let Some(c) = next_char(bytes, idx) {
                    if c == b';' {
                        break;
                    }
                }
                let num_str =
                    std::str::from_utf8(&bytes[start..*idx - 1]).map_err(|_| "invalid utf8")?;
                let i: i64 = num_str.parse().map_err(|_| "invalid int")?;
                Ok(SerializedTok::Int(i))
            }
            Some(b'd') => {
                if next_char(bytes, idx) != Some(b':') {
                    return Err("expected :".into());
                }
                let start = *idx;
                while let Some(c) = next_char(bytes, idx) {
                    if c == b';' {
                        break;
                    }
                }
                let num_str =
                    std::str::from_utf8(&bytes[start..*idx - 1]).map_err(|_| "invalid utf8")?;
                let f: f64 = num_str.parse().map_err(|_| "invalid float")?;
                Ok(SerializedTok::Float(f))
            }
            Some(b'b') => {
                if next_char(bytes, idx) != Some(b':') {
                    return Err("expected :".into());
                }
                let val = next_char(bytes, idx).ok_or("unexpected eof")?;
                if next_char(bytes, idx) != Some(b';') {
                    return Err("expected ;".into());
                }
                match val {
                    b'0' => Ok(SerializedTok::Bool(false)),
                    b'1' => Ok(SerializedTok::Bool(true)),
                    _ => Err("invalid bool".into()),
                }
            }
            Some(b'N') => {
                if next_char(bytes, idx) != Some(b';') {
                    return Err("expected ;".into());
                }
                Ok(SerializedTok::Null)
            }
            Some(b'a') => {
                if next_char(bytes, idx) != Some(b':') {
                    return Err("expected :".into());
                }
                let start = *idx;
                while let Some(c) = next_char(bytes, idx) {
                    if c == b':' {
                        break;
                    }
                }
                let len_str =
                    std::str::from_utf8(&bytes[start..*idx - 1]).map_err(|_| "invalid utf8")?;
                let _len: usize = len_str.parse().map_err(|_| "invalid length")?;
                if next_char(bytes, idx) != Some(b'{') {
                    return Err("expected {".into());
                }
                let mut entries = Vec::new();
                loop {
                    if let Some(b'}') = bytes.get(*idx) {
                        *idx += 1;
                        break;
                    }
                    let key = parse_value(bytes, idx)?;
                    let val = parse_value(bytes, idx)?;
                    entries.push((key, val));
                }
                Ok(SerializedTok::Array(entries))
            }
            _ => Err("unsupported token".into()),
        }
    }

    let val = parse_value(bytes, &mut idx)?;
    Ok(tok_to_json(&val))
}

fn tok_to_json(tok: &SerializedTok<'_>) -> JsonValue {
    match tok {
        SerializedTok::Str(s) => JsonValue::String((*s).to_string()),
        SerializedTok::Int(i) => JsonValue::Number((*i).into()),
        SerializedTok::Float(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        SerializedTok::Bool(b) => JsonValue::Bool(*b),
        SerializedTok::Null => JsonValue::Null,
        SerializedTok::Array(entries) => {
            let is_list = entries
                .iter()
                .enumerate()
                .all(|(idx, (k, _))| matches!(k, SerializedTok::Int(i) if *i as usize == idx));
            if is_list {
                JsonValue::Array(entries.iter().map(|(_, v)| tok_to_json(v)).collect())
            } else {
                let mut map = serde_json::Map::new();
                for (k, v) in entries {
                    let key = match k {
                        SerializedTok::Str(s) => (*s).to_string(),
                        SerializedTok::Int(i) => i.to_string(),
                        _ => format!("{k:?}"),
                    };
                    map.insert(key, tok_to_json(v));
                }
                JsonValue::Object(map)
            }
        }
    }
}
