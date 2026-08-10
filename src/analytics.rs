use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::Serialize;
use sqlx::{FromRow, MySqlPool};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CirculationPoint {
    pub period: String,
    pub new_loans: i64,
    pub returns: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct DdcPoint {
    pub ddc_class: String,
    pub biblio_count: i64,
    pub item_count: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct TopBook {
    pub biblio_id: i64,
    pub title: String,
    pub gmd_name: Option<String>,
    pub loan_count: i64,
}

#[derive(Debug, FromRow)]
struct PeriodCount {
    period: String,
    count: i64,
}

pub fn date_format(group_by: &str) -> &'static str {
    match group_by {
        "week" => "%Y-%u",
        "month" => "%Y-%m",
        _ => "%Y-%m-%d",
    }
}

pub async fn circulation_trend(
    pool: &MySqlPool,
    start: NaiveDate,
    end: NaiveDate,
    group_by: &str,
) -> Result<Vec<CirculationPoint>, sqlx::Error> {
    let format = date_format(group_by);
    let loans_sql = format!(
        "SELECT DATE_FORMAT(loan_date, '{format}') AS period, COUNT(*) AS count \
         FROM loan WHERE DATE(loan_date) BETWEEN ? AND ? GROUP BY period ORDER BY period"
    );
    let returns_sql = format!(
        "SELECT DATE_FORMAT(return_date, '{format}') AS period, COUNT(*) AS count \
         FROM loan WHERE is_return = 1 AND DATE(return_date) BETWEEN ? AND ? \
         GROUP BY period ORDER BY period"
    );

    let loans = sqlx::query_as::<_, PeriodCount>(&loans_sql)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await?;
    let returns = sqlx::query_as::<_, PeriodCount>(&returns_sql)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await?;

    let mut points = BTreeMap::<String, (i64, i64)>::new();
    for row in loans {
        points.entry(row.period).or_default().0 = row.count;
    }
    for row in returns {
        points.entry(row.period).or_default().1 = row.count;
    }

    Ok(points
        .into_iter()
        .map(|(period, (new_loans, returns))| CirculationPoint {
            period,
            new_loans,
            returns,
        })
        .collect())
}

pub async fn collection_by_ddc(pool: &MySqlPool, level: u8) -> Result<Vec<DdcPoint>, sqlx::Error> {
    let class_expression = if level == 3 {
        "LEFT(b.classification, 3)"
    } else {
        "CONCAT(LEFT(b.classification, 1), 'xx')"
    };
    let sql = format!(
        "SELECT {class_expression} AS ddc_class, \
         COUNT(DISTINCT b.biblio_id) AS biblio_count, COUNT(i.item_id) AS item_count \
         FROM biblio b LEFT JOIN item i ON i.biblio_id = b.biblio_id \
         WHERE b.classification IS NOT NULL AND b.classification != '' \
         GROUP BY ddc_class ORDER BY biblio_count DESC"
    );
    sqlx::query_as::<_, DdcPoint>(&sql).fetch_all(pool).await
}

pub async fn top_books(
    pool: &MySqlPool,
    start: NaiveDate,
    end: NaiveDate,
    limit: i64,
) -> Result<Vec<TopBook>, sqlx::Error> {
    sqlx::query_as::<_, TopBook>(
        "SELECT b.biblio_id, b.title, g.gmd_name, COUNT(l.loan_id) AS loan_count \
         FROM loan l JOIN item i ON l.item_code = i.item_code \
         JOIN biblio b ON i.biblio_id = b.biblio_id \
         LEFT JOIN mst_gmd g ON b.gmd_id = g.gmd_id \
         WHERE DATE(l.loan_date) BETWEEN ? AND ? \
         GROUP BY b.biblio_id, b.title, g.gmd_name \
         ORDER BY loan_count DESC LIMIT ?",
    )
    .bind(start)
    .bind(end)
    .bind(limit.clamp(1, 50))
    .fetch_all(pool)
    .await
}
