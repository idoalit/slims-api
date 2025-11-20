use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
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
        bind_filters_to_query, bind_filters_to_scalar, where_clause, FilterField, FilterOperator,
        FilterValueType, ListParams, SortField,
    },
};

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Loan {
    pub loan_id: i64,
    pub item_code: Option<String>,
    pub member_id: Option<String>,
    pub loan_date: NaiveDate,
    pub due_date: NaiveDate,
    pub actual: Option<NaiveDate>,
    pub return_date: Option<NaiveDate>,
    pub is_return: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLoan {
    pub item_code: String,
    pub member_id: String,
    pub due_date: NaiveDate,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct LoanMember {
    pub member_id: String,
    pub member_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, ToSchema)]
pub struct LoanItem {
    pub item_id: i64,
    pub item_code: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoanResponse {
    #[serde(flatten)]
    pub loan: Loan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<LoanMember>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<LoanItem>,
}

const LOAN_SORTS: &[SortField<'_>] = &[
    SortField::new("loan_date", "loan.loan_date"),
    SortField::new("due_date", "loan.due_date"),
    SortField::new("return_date", "loan.return_date"),
    SortField::new("loan_id", "loan.loan_id"),
];

const LOAN_FILTERS: &[FilterField<'_>] = &[
    FilterField::new(
        "item_code",
        "loan.item_code",
        FilterOperator::Equals,
        FilterValueType::Text,
    ),
    FilterField::new(
        "member_id",
        "loan.member_id",
        FilterOperator::Equals,
        FilterValueType::Text,
    ),
    FilterField::new(
        "is_return",
        "loan.is_return",
        FilterOperator::Equals,
        FilterValueType::Boolean,
    ),
];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_loans).post(create_loan))
        .route("/:loan_id/return", post(return_loan))
}

#[utoipa::path(
    get,
    path = "/loans",
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Loans"
)]
async fn list_loans(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Circulation, Permission::Read)?;

    let pagination = params.pagination();
    let includes = params.includes();
    let loan_fields = params.fieldset("loans");
    let (limit, offset, page, per_page) = pagination.limit_offset();
    let sort_clause = params.sort_clause(LOAN_SORTS, "loan.loan_date DESC")?;
    let filters = params.filter_clauses(LOAN_FILTERS)?;
    let where_sql = where_clause(&filters);

    let count_sql = format!("SELECT COUNT(*) FROM loan {}", where_sql);
    let total = bind_filters_to_scalar(sqlx::query_scalar::<_, i64>(&count_sql), &filters)
        .fetch_one(&state.pool)
        .await?;

    let data_sql = format!(
        "SELECT loan_id, item_code, member_id, loan_date, due_date, actual, return_date, is_return FROM loan {} ORDER BY {} LIMIT ? OFFSET ?",
        where_sql, sort_clause
    );
    let loans = bind_filters_to_query(sqlx::query_as::<_, Loan>(&data_sql), &filters)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let mut member_cache: HashMap<String, LoanMember> = HashMap::new();
    let mut item_cache: HashMap<String, LoanItem> = HashMap::new();
    let mut data = Vec::with_capacity(loans.len());

    for loan in loans {
        let mut member = None;
        if includes.contains("member") {
            if let Some(member_id) = loan.member_id.clone() {
                if let Some(existing) = member_cache.get(&member_id) {
                    member = Some(existing.clone());
                } else if let Some(row) = sqlx::query_as::<_, LoanMember>(
                    "SELECT member_id, member_name FROM member WHERE member_id = ?",
                )
                .bind(&member_id)
                .fetch_optional(&state.pool)
                .await?
                {
                    member_cache.insert(member_id.clone(), row.clone());
                    member = Some(row);
                }
            }
        }

        let mut item = None;
        if includes.contains("item") {
            if let Some(code) = loan.item_code.clone() {
                if let Some(existing) = item_cache.get(&code) {
                    item = Some(existing.clone());
                } else if let Some(row) = sqlx::query_as::<_, LoanItem>(
                    "SELECT item_id, item_code FROM item WHERE item_code = ?",
                )
                .bind(&code)
                .fetch_optional(&state.pool)
                .await?
                {
                    item_cache.insert(code.clone(), row.clone());
                    item = Some(row);
                }
            }
        }

        let response = LoanResponse { loan, member, item };
        data.push(resource_with_fields(
            "loans",
            response.loan.loan_id.to_string(),
            response,
            loan_fields,
        ));
    }

    Ok(Json(collection_document(
        data,
        pagination_meta(page, per_page, total),
    )))
}

#[utoipa::path(
    post,
    path = "/loans",
    request_body = CreateLoan,
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Loans"
)]
async fn create_loan(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateLoan>,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Circulation, Permission::Write)?;

    let today = chrono::Utc::now().date_naive();

    let result = sqlx::query(
        "INSERT INTO loan (item_code, member_id, loan_date, due_date, is_lent, is_return) VALUES (?, ?, ?, ?, 1, 0)",
    )
    .bind(&payload.item_code)
    .bind(&payload.member_id)
    .bind(today)
    .bind(payload.due_date)
    .execute(&state.pool)
    .await?;

    let rec = sqlx::query_as::<_, Loan>(
        "SELECT loan_id, item_code, member_id, loan_date, due_date, actual, return_date, is_return FROM loan WHERE loan_id = ?",
    )
    .bind(result.last_insert_id() as i64)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(single_document(resource(
        "loans",
        rec.loan_id.to_string(),
        rec,
    ))))
}

#[utoipa::path(
    post,
    path = "/loans/{loan_id}/return",
    params(("loan_id" = i64, Path, description = "Loan ID")),
    responses((status = 200, body = JsonApiDocument)),
    security(("bearerAuth" = [])),
    tag = "Loans"
)]
async fn return_loan(
    State(state): State<AppState>,
    Path(loan_id): Path<i64>,
    auth: AuthUser,
) -> Result<Json<JsonApiDocument>, AppError> {
    auth.require_access(ModuleAccess::Circulation, Permission::Write)?;

    let today = chrono::Utc::now().date_naive();

    let updated =
        sqlx::query("UPDATE loan SET return_date = ?, is_return = 1, actual = ? WHERE loan_id = ?")
            .bind(today)
            .bind(today)
            .bind(loan_id)
            .execute(&state.pool)
            .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let rec = sqlx::query_as::<_, Loan>(
        "SELECT loan_id, item_code, member_id, loan_date, due_date, actual, return_date, is_return FROM loan WHERE loan_id = ?",
    )
    .bind(loan_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(single_document(resource(
        "loans",
        rec.loan_id.to_string(),
        rec,
    ))))
}
