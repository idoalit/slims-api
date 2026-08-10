use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    analytics::{self, CirculationPoint, DdcPoint, TopBook},
    auth::{AuthUser, ModuleAccess, Permission},
    config::AppState,
    error::AppError,
    jsonapi::{JsonApiDocument, resource, single_document},
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct DashboardParams {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub group_by: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DashboardPeriod {
    pub start_date: String,
    pub end_date: String,
    pub group_by: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DashboardMetrics {
    pub total_biblios: Option<i64>,
    pub total_items: Option<i64>,
    pub total_members: Option<i64>,
    pub active_loans: Option<i64>,
    pub overdue_loans: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DashboardResponse {
    pub period: DashboardPeriod,
    pub metrics: DashboardMetrics,
    pub circulation_trend: Option<Vec<CirculationPoint>>,
    pub collection_by_ddc: Option<Vec<DdcPoint>>,
    pub top_books: Option<Vec<TopBook>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_dashboard))
}

fn parse_period(params: DashboardParams) -> Result<(NaiveDate, NaiveDate, String), AppError> {
    let today = Utc::now().date_naive();
    let end = params
        .end_date
        .map(|value| NaiveDate::parse_from_str(&value, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| AppError::BadRequest("end_date must use YYYY-MM-DD".into()))?
        .unwrap_or(today);
    let start = params
        .start_date
        .map(|value| NaiveDate::parse_from_str(&value, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| AppError::BadRequest("start_date must use YYYY-MM-DD".into()))?
        .unwrap_or(end - Duration::days(29));

    if start > end {
        return Err(AppError::BadRequest(
            "start_date must not be after end_date".into(),
        ));
    }
    if (end - start).num_days() > 365 {
        return Err(AppError::BadRequest(
            "dashboard period cannot exceed 366 days".into(),
        ));
    }

    let group_by = params.group_by.unwrap_or_else(|| "day".into());
    if !matches!(group_by.as_str(), "day" | "week" | "month") {
        return Err(AppError::BadRequest(
            "group_by must be day, week, or month".into(),
        ));
    }
    Ok((start, end, group_by))
}

#[utoipa::path(
    get,
    path = "/dashboard",
    params(
        ("start_date" = Option<String>, Query, description = "YYYY-MM-DD"),
        ("end_date" = Option<String>, Query, description = "YYYY-MM-DD"),
        ("group_by" = Option<String>, Query, description = "day, week, or month")
    ),
    responses((status = 200, body = JsonApiDocument), (status = 400), (status = 401)),
    security(("bearerAuth" = [])),
    tag = "Dashboard"
)]
pub async fn get_dashboard(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<DashboardParams>,
) -> Result<Json<JsonApiDocument>, AppError> {
    let (start, end, group_by) = parse_period(params)?;
    let bibliography = auth.can_access(ModuleAccess::Bibliography, Permission::Read);
    let membership = auth.can_access(ModuleAccess::Membership, Permission::Read);
    let circulation = auth.can_access(ModuleAccess::Circulation, Permission::Read);

    let (total_biblios, total_items, collection_by_ddc) = if bibliography {
        let biblios = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM biblio")
            .fetch_one(&state.pool)
            .await?;
        let items = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM item")
            .fetch_one(&state.pool)
            .await?;
        let ddc = analytics::collection_by_ddc(&state.pool, 1).await?;
        (Some(biblios), Some(items), Some(ddc))
    } else {
        (None, None, None)
    };

    let total_members = if membership {
        Some(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM member")
                .fetch_one(&state.pool)
                .await?,
        )
    } else {
        None
    };

    let (active_loans, overdue_loans, circulation_trend, top_books) = if circulation {
        let active = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM loan WHERE is_return = 0")
            .fetch_one(&state.pool)
            .await?;
        let overdue = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM loan WHERE is_return = 0 AND due_date < CURDATE()",
        )
        .fetch_one(&state.pool)
        .await?;
        let trend = analytics::circulation_trend(&state.pool, start, end, &group_by).await?;
        let books = analytics::top_books(&state.pool, start, end, 5).await?;
        (Some(active), Some(overdue), Some(trend), Some(books))
    } else {
        (None, None, None, None)
    };

    let response = DashboardResponse {
        period: DashboardPeriod {
            start_date: start.to_string(),
            end_date: end.to_string(),
            group_by,
        },
        metrics: DashboardMetrics {
            total_biblios,
            total_items,
            total_members,
            active_loans,
            overdue_loans,
        },
        circulation_trend,
        collection_by_ddc,
        top_books,
    };

    Ok(Json(single_document(resource(
        "dashboard",
        "current",
        response,
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_periods() {
        let result = parse_period(DashboardParams {
            start_date: Some("2026-02-02".into()),
            end_date: Some("2026-02-01".into()),
            group_by: Some("day".into()),
        });
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn accepts_valid_period_and_grouping() {
        let (start, end, group) = parse_period(DashboardParams {
            start_date: Some("2026-01-01".into()),
            end_date: Some("2026-01-31".into()),
            group_by: Some("week".into()),
        })
        .unwrap();
        assert_eq!(
            (start.to_string(), end.to_string(), group),
            ("2026-01-01".into(), "2026-01-31".into(), "week".into())
        );
    }

    #[test]
    fn rejects_periods_longer_than_366_days() {
        let result = parse_period(DashboardParams {
            start_date: Some("2025-01-01".into()),
            end_date: Some("2026-01-02".into()),
            group_by: Some("month".into()),
        });
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn rejects_unknown_grouping() {
        let result = parse_period(DashboardParams {
            start_date: Some("2026-01-01".into()),
            end_date: Some("2026-01-31".into()),
            group_by: Some("year".into()),
        });
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn restricted_dashboard_fields_serialize_as_null() {
        let response = DashboardResponse {
            period: DashboardPeriod {
                start_date: "2026-01-01".into(),
                end_date: "2026-01-30".into(),
                group_by: "day".into(),
            },
            metrics: DashboardMetrics {
                total_biblios: None,
                total_items: None,
                total_members: None,
                active_loans: None,
                overdue_loans: None,
            },
            circulation_trend: None,
            collection_by_ddc: None,
            top_books: None,
        };
        let value = serde_json::to_value(response).unwrap();

        assert!(value["metrics"]["total_biblios"].is_null());
        assert!(value["metrics"]["total_members"].is_null());
        assert!(value["circulation_trend"].is_null());
        assert!(value["collection_by_ddc"].is_null());
        assert!(value["top_books"].is_null());
    }
}
