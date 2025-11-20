pub mod biblios;
pub mod contents;
pub mod files;
pub mod items;
pub mod loans;
pub mod lookups;
pub mod members;
pub mod settings;
pub mod visitors;

use serde::Deserialize;
use sqlx::{
    mysql::MySqlArguments,
    query::{QueryAs, QueryScalar},
    MySql,
};
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PER_PAGE: u32 = 20;
const MAX_PER_PAGE: u32 = 100;

#[derive(Debug, Default, Deserialize, Clone, Copy, ToSchema)]
pub struct Pagination {
    #[serde(rename = "page[number]", alias = "page")]
    pub page_number: Option<u32>,
    #[serde(rename = "page[size]", alias = "per_page")]
    pub page_size: Option<u32>,
}

impl Pagination {
    fn resolved(&self) -> (u32, u32) {
        let page = self.page_number.unwrap_or(DEFAULT_PAGE).max(1);
        let per_page = self
            .page_size
            .unwrap_or(DEFAULT_PER_PAGE)
            .clamp(1, MAX_PER_PAGE);
        (page, per_page)
    }

    pub fn limit_offset(&self) -> (i64, i64, u32, u32) {
        let (page, per_page) = self.resolved();
        let offset = ((page - 1) * per_page) as i64;
        (per_page as i64, offset, page, per_page)
    }
}

#[derive(Debug, Clone, ToSchema)]
pub struct ListParams {
    pagination: Pagination,
    pub include: Option<String>,
    fields: HashMap<String, HashSet<String>>,
    filters: HashMap<String, Vec<String>>,
    sorts: Vec<SortOrder>,
}

impl<'de> Deserialize<'de> for ListParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawParams {
            #[serde(flatten)]
            pagination: Pagination,
            #[serde(default)]
            include: Option<String>,
            #[serde(default)]
            sort: Option<String>,
            #[serde(flatten)]
            extras: HashMap<String, String>,
        }

        let raw = RawParams::deserialize(deserializer)?;

        let mut fields: HashMap<String, HashSet<String>> = HashMap::new();
        let mut filters: HashMap<String, Vec<String>> = HashMap::new();

        for (key, value) in raw.extras {
            if let Some(name) = key.strip_prefix("fields[") {
                if let Some(name) = name.strip_suffix(']') {
                    let set = value
                        .split(',')
                        .filter_map(|part| {
                            let trimmed = part.trim();
                            (!trimmed.is_empty()).then(|| trimmed.to_string())
                        })
                        .collect::<HashSet<_>>();
                    if !set.is_empty() {
                        fields.insert(name.to_string(), set);
                    }
                    continue;
                }
            }

            if let Some(name) = key.strip_prefix("filter[") {
                if let Some(name) = name.strip_suffix(']') {
                    let values = value
                        .split(',')
                        .filter_map(|part| {
                            let trimmed = part.trim();
                            (!trimmed.is_empty()).then(|| trimmed.to_string())
                        })
                        .collect::<Vec<_>>();
                    if !values.is_empty() {
                        filters.insert(name.to_string(), values);
                    }
                }
            }
        }

        let sorts = raw
            .sort
            .as_deref()
            .map(parse_sort_string)
            .unwrap_or_default();

        Ok(ListParams {
            pagination: raw.pagination,
            include: raw.include,
            fields,
            filters,
            sorts,
        })
    }
}

impl ListParams {
    pub fn pagination(&self) -> Pagination {
        self.pagination
    }

    pub fn includes(&self) -> HashSet<String> {
        parse_include(self.include.clone())
    }

    pub fn fieldset(&self, resource_type: &str) -> Option<&HashSet<String>> {
        self.fields.get(resource_type)
    }

    pub fn sort_clause(
        &self,
        allowed: &[SortField<'_>],
        default: &str,
    ) -> Result<String, crate::error::AppError> {
        if self.sorts.is_empty() {
            return Ok(default.to_string());
        }

        let mut clauses = Vec::with_capacity(self.sorts.len());
        for order in &self.sorts {
            if let Some(def) = allowed.iter().find(|def| def.name == order.field) {
                let direction = if order.ascending { "ASC" } else { "DESC" };
                clauses.push(format!("{} {}", def.column, direction));
            } else {
                return Err(crate::error::AppError::BadRequest(format!(
                    "sorting by `{}` is not supported",
                    order.field
                )));
            }
        }

        Ok(clauses.join(", "))
    }

    pub fn filter_clauses(
        &self,
        allowed: &[FilterField<'_>],
    ) -> Result<Vec<FilterClause>, crate::error::AppError> {
        let mut clauses = Vec::new();
        for (name, values) in &self.filters {
            let def = allowed
                .iter()
                .find(|item| item.name == name)
                .ok_or_else(|| {
                    crate::error::AppError::BadRequest(format!(
                        "filter `{}` is not supported",
                        name
                    ))
                })?;

            if values.len() > 1 {
                return Err(crate::error::AppError::BadRequest(format!(
                    "multiple filter values for `{}` are not supported",
                    name
                )));
            }

            let raw_value = values.first().expect("checked non-empty");
            let (statement, value) = def.to_clause(raw_value)?;
            clauses.push(FilterClause { statement, value });
        }
        Ok(clauses)
    }
}

fn parse_sort_string(raw: &str) -> Vec<SortOrder> {
    raw.split(',')
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return None;
            }

            let (ascending, field) = if let Some(field) = trimmed.strip_prefix('-') {
                (false, field)
            } else if let Some(field) = trimmed.strip_prefix('+') {
                (true, field)
            } else {
                (true, trimmed)
            };

            Some(SortOrder {
                field: field.to_string(),
                ascending,
            })
        })
        .collect()
}

pub fn parse_include(raw: Option<String>) -> HashSet<String> {
    raw.map(|s| {
        s.split(',')
            .filter_map(|part| {
                let trimmed = part.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_lowercase())
            })
            .collect()
    })
    .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct SortOrder {
    pub field: String,
    pub ascending: bool,
}

#[derive(Clone, Copy)]
pub struct SortField<'a> {
    pub name: &'a str,
    pub column: &'a str,
}

impl<'a> SortField<'a> {
    pub const fn new(name: &'a str, column: &'a str) -> Self {
        SortField { name, column }
    }
}

#[derive(Clone, Copy)]
pub enum FilterOperator {
    Equals,
    Like,
}

#[derive(Clone, Copy)]
pub enum FilterValueType {
    Text,
    Integer,
    Boolean,
}

#[derive(Clone, Copy)]
pub struct FilterField<'a> {
    pub name: &'a str,
    pub column: &'a str,
    pub operator: FilterOperator,
    pub value_type: FilterValueType,
}

impl<'a> FilterField<'a> {
    pub const fn new(
        name: &'a str,
        column: &'a str,
        operator: FilterOperator,
        value_type: FilterValueType,
    ) -> Self {
        FilterField {
            name,
            column,
            operator,
            value_type,
        }
    }

    fn to_clause(
        &self,
        raw_value: &str,
    ) -> Result<(String, FilterValue), crate::error::AppError> {
        let (statement, value) = match self.operator {
            FilterOperator::Equals => {
                let value = self.parse_value(raw_value)?;
                (format!("{} = ?", self.column), value)
            }
            FilterOperator::Like => {
                let value = FilterValue::Text(format!("%{}%", raw_value));
                (format!("{} LIKE ?", self.column), value)
            }
        };
        Ok((statement, value))
    }

    fn parse_value(
        &self,
        raw_value: &str,
    ) -> Result<FilterValue, crate::error::AppError> {
        match self.value_type {
            FilterValueType::Text => Ok(FilterValue::Text(raw_value.to_string())),
            FilterValueType::Integer => raw_value
                .parse::<i64>()
                .map(FilterValue::Integer)
                .map_err(|_| {
                    crate::error::AppError::BadRequest(format!(
                        "filter `{}` must be an integer",
                        self.name
                    ))
                }),
            FilterValueType::Boolean => match raw_value {
                "true" | "1" => Ok(FilterValue::Boolean(true)),
                "false" | "0" => Ok(FilterValue::Boolean(false)),
                _ => Err(crate::error::AppError::BadRequest(format!(
                    "filter `{}` must be boolean",
                    self.name
                ))),
            },
        }
    }
}

#[derive(Clone)]
pub enum FilterValue {
    Text(String),
    Integer(i64),
    Boolean(bool),
}

impl FilterValue {
    fn bind_query<'q, T>(
        &self,
        query: QueryAs<'q, MySql, T, MySqlArguments>,
    ) -> QueryAs<'q, MySql, T, MySqlArguments> {
        match self {
            FilterValue::Text(val) => query.bind(val.clone()),
            FilterValue::Integer(val) => query.bind(*val),
            FilterValue::Boolean(val) => query.bind(*val),
        }
    }

    fn bind_scalar<'q, T>(
        &self,
        query: QueryScalar<'q, MySql, T, MySqlArguments>,
    ) -> QueryScalar<'q, MySql, T, MySqlArguments> {
        match self {
            FilterValue::Text(val) => query.bind(val.clone()),
            FilterValue::Integer(val) => query.bind(*val),
            FilterValue::Boolean(val) => query.bind(*val),
        }
    }

}

#[derive(Clone)]
pub struct FilterClause {
    pub statement: String,
    pub value: FilterValue,
}

pub fn where_clause(filters: &[FilterClause]) -> String {
    if filters.is_empty() {
        String::new()
    } else {
        let mut combined = String::from("WHERE ");
        combined.push_str(
            &filters
                .iter()
                .map(|clause| clause.statement.as_str())
                .collect::<Vec<_>>()
                .join(" AND "),
        );
        combined
    }
}

pub fn bind_filters_to_query<'q, T>(
    mut query: QueryAs<'q, MySql, T, MySqlArguments>,
    filters: &[FilterClause],
) -> QueryAs<'q, MySql, T, MySqlArguments> {
    for clause in filters {
        query = clause.value.bind_query(query);
    }
    query
}

pub fn bind_filters_to_scalar<'q, T>(
    mut query: QueryScalar<'q, MySql, T, MySqlArguments>,
    filters: &[FilterClause],
) -> QueryScalar<'q, MySql, T, MySqlArguments> {
    for clause in filters {
        query = clause.value.bind_scalar(query);
    }
    query
}
