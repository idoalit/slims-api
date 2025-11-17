use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;

use crate::{
    auth::{AuthUser, Role},
    config::AppState,
    error::AppError,
    resources::{ListParams, PagedResponse},
};

#[derive(Debug, Serialize, Deserialize, FromRow)]
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

#[derive(Debug, Deserialize)]
pub struct CreateLoan {
    pub item_code: String,
    pub member_id: String,
    pub due_date: NaiveDate,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct LoanMember {
    pub member_id: String,
    pub member_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct LoanItem {
    pub item_id: i64,
    pub item_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoanResponse {
    #[serde(flatten)]
    pub loan: Loan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<LoanMember>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<LoanItem>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_loans).post(create_loan))
        .route("/:loan_id/return", post(return_loan))
}

async fn list_loans(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<PagedResponse<LoanResponse>>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff])?;

    let pagination = params.pagination();
    let includes = params.includes();
    let (limit, offset, page, per_page) = pagination.limit_offset();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM loan")
        .fetch_one(&state.pool)
        .await?;

    let loans = sqlx::query_as::<_, Loan>(
        "SELECT loan_id, item_code, member_id, loan_date, due_date, actual, return_date, is_return FROM loan ORDER BY loan_date DESC LIMIT ? OFFSET ?",
    )
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
                } else if let Some(row) =
                    sqlx::query_as::<_, LoanItem>("SELECT item_id, item_code FROM item WHERE item_code = ?")
                        .bind(&code)
                        .fetch_optional(&state.pool)
                        .await?
                {
                    item_cache.insert(code.clone(), row.clone());
                    item = Some(row);
                }
            }
        }

        data.push(LoanResponse { loan, member, item });
    }

    Ok(Json(PagedResponse {
        data,
        page,
        per_page,
        total,
    }))
}

async fn create_loan(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateLoan>,
) -> Result<Json<Loan>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff])?;

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

    Ok(Json(rec))
}

async fn return_loan(
    State(state): State<AppState>,
    Path(loan_id): Path<i64>,
    auth: AuthUser,
) -> Result<Json<Loan>, AppError> {
    auth.require_roles(&[Role::Admin, Role::Librarian, Role::Staff])?;

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

    Ok(Json(rec))
}
