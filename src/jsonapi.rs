use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct JsonApiDocument {
    #[schema(value_type = Object)]
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object, nullable)]
    pub meta: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Vec<Object>, nullable)]
    pub included: Option<Vec<Value>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JsonApiError {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JsonApiErrorDocument {
    pub errors: Vec<JsonApiError>,
}

pub fn resource<T: Serialize>(
    resource_type: &'static str,
    id: impl Into<String>,
    attributes: T,
) -> Value {
    resource_with_fields(resource_type, id, attributes, None)
}

pub fn resource_with_fields<T: Serialize>(
    resource_type: &'static str,
    id: impl Into<String>,
    attributes: T,
    fields: Option<&HashSet<String>>,
) -> Value {
    let mut value = serde_json::to_value(attributes).unwrap_or(Value::Null);
    if let (Some(allowed), Value::Object(map)) = (fields, &mut value) {
        map.retain(|key, _| allowed.contains(key));
    }

    json!({
        "type": resource_type,
        "id": id.into(),
        "attributes": value,
    })
}

pub fn single_document(resource: Value) -> JsonApiDocument {
    JsonApiDocument {
        data: resource,
        meta: None,
        included: None,
    }
}

pub fn collection_document(data: Vec<Value>, meta: Value) -> JsonApiDocument {
    JsonApiDocument {
        data: Value::Array(data),
        meta: Some(meta),
        included: None,
    }
}

pub fn pagination_meta(page: u32, per_page: u32, total: i64) -> Value {
    json!({
        "page": page,
        "per_page": per_page,
        "total": total,
    })
}
