use chrono::Datelike as _;
use rmcp::{ErrorData as McpError, handler::server::wrapper::Parameters, tool, tool_router};
use serde::Serialize;

use super::{LibraryMcpServer, types::*};
use crate::analytics;

#[tool_router(router = stats_tool_router, vis = "pub(crate)")]
impl LibraryMcpServer {
    // ─── 1. Tren Sirkulasi (Peminjaman vs Pengembalian) ───────────────────────

    /// Data tren peminjaman baru vs pengembalian per hari/minggu/bulan.
    /// Cocok untuk line chart perbandingan sirkulasi.
    #[tool(
        description = "Circulation trend data (new loans vs returns) grouped by day, week, or month. Returns a time-series array suitable for a line chart. group_by: \"day\" (default) | \"week\" | \"month\"."
    )]
    async fn library_stats_circulation_trend(
        &self,
        Parameters(input): Parameters<StatsPeriodInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();
        let default_start = (today - chrono::Duration::days(30)).to_string();
        let default_end = today.to_string();
        let start = input
            .start_date
            .as_deref()
            .unwrap_or(&default_start)
            .to_owned();
        let end = input.end_date.as_deref().unwrap_or(&default_end).to_owned();
        let group_by = input.group_by.as_deref().unwrap_or("day");
        let start_date = chrono::NaiveDate::parse_from_str(&start, "%Y-%m-%d")
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let end_date = chrono::NaiveDate::parse_from_str(&end, "%Y-%m-%d")
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let trend = analytics::circulation_trend(&self.pool, start_date, end_date, group_by)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let total_new_loans: i64 = trend.iter().map(|row| row.new_loans).sum();
        let total_returns: i64 = trend.iter().map(|row| row.returns).sum();

        #[derive(Serialize)]
        struct CircSummary {
            total_new_loans: i64,
            total_returns: i64,
        }
        #[derive(Serialize)]
        struct CircTrendReport {
            period: String,
            group_by: String,
            summary: CircSummary,
            trend: Vec<analytics::CirculationPoint>,
        }

        serde_json::to_string_pretty(&CircTrendReport {
            period: format!("{} s/d {}", start, end),
            group_by: group_by.to_owned(),
            summary: CircSummary {
                total_new_loans,
                total_returns,
            },
            trend,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 2. Kunjungan Harian/Bulanan ──────────────────────────────────────────

    /// Data tren kunjungan perpustakaan per hari/minggu/bulan dari tabel visitor_count.
    /// Cocok untuk line chart kepadatan kunjungan.
    #[tool(
        description = "Visitor trend data grouped by day, week, or month — suitable for a line chart. group_by: \"day\" (default) | \"week\" | \"month\"."
    )]
    async fn library_stats_visitor_trend(
        &self,
        Parameters(input): Parameters<StatsPeriodInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();
        let default_start = (today - chrono::Duration::days(30)).to_string();
        let default_end = today.to_string();
        let start = input
            .start_date
            .as_deref()
            .unwrap_or(&default_start)
            .to_owned();
        let end = input.end_date.as_deref().unwrap_or(&default_end).to_owned();
        let group_by = input.group_by.as_deref().unwrap_or("day");
        let fmt = date_format_for(group_by);

        let sql = format!(
            "SELECT DATE_FORMAT(checkin_date, '{fmt}') as period, COUNT(*) as count \
             FROM visitor_count WHERE DATE(checkin_date) BETWEEN ? AND ? \
             GROUP BY period ORDER BY period",
            fmt = fmt
        );
        let rows = sqlx::query_as::<_, PeriodCountRow>(&sql)
            .bind(&start)
            .bind(&end)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let total_visits: i64 = rows.iter().map(|r| r.count).sum();

        #[derive(Serialize)]
        struct VisitorTrendReport {
            period: String,
            group_by: String,
            total_visits: i64,
            trend: Vec<PeriodCountRow>,
        }

        serde_json::to_string_pretty(&VisitorTrendReport {
            period: format!("{} s/d {}", start, end),
            group_by: group_by.to_owned(),
            total_visits,
            trend: rows,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 3. Komposisi Koleksi berdasarkan Klasifikasi DDC ────────────────────

    /// Distribusi koleksi berdasarkan kelas DDC (Dewey Decimal Classification).
    /// Level 1 menampilkan 10 kelas utama (0xx–9xx), level 3 menampilkan subkelas.
    /// Cocok untuk pie chart atau bar chart komposisi DDC.
    #[tool(
        description = "Collection composition by DDC classification. level=1 (default) shows main classes (0xx–9xx), level=3 shows full DDC subclasses. Suitable for a pie or bar chart."
    )]
    async fn library_stats_collection_by_ddc(
        &self,
        Parameters(input): Parameters<DdcInput>,
    ) -> Result<String, McpError> {
        let level = input.level.unwrap_or(1).clamp(1, 3) as usize;

        let rows = analytics::collection_by_ddc(&self.pool, level as u8)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct DdcReport {
            level: usize,
            description: &'static str,
            data: Vec<analytics::DdcPoint>,
        }

        serde_json::to_string_pretty(&DdcReport {
            level,
            description: if level == 1 {
                "Main DDC classes (0xx–9xx)"
            } else {
                "DDC subclasses (3-digit)"
            },
            data: rows,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 4. Perbandingan Jenis Media: Fisik vs Digital ───────────────────────

    /// Perbandingan koleksi fisik vs digital berdasarkan jenis GMD.
    /// Digital: Computer File (CF), Electronic Resource (ER), Computer Software (CO),
    /// Music (MU), Sound Recording (SO), Video Recording (VI).
    /// Cocok untuk pie chart atau stacked bar.
    #[tool(
        description = "Physical vs digital collection comparison based on GMD type. Digital GMDs: CF, ER, CO, MU, SO, VI. Returns biblio_count and item_count per media type — suitable for pie chart."
    )]
    async fn library_stats_media_type_comparison(&self) -> Result<String, McpError> {
        let rows = sqlx::query_as::<_, MediaTypeRow>(
            "SELECT \
                CASE WHEN UPPER(g.gmd_code) IN ('CF','ER','CO','MU','SO','VI') \
                     THEN 'Digital' ELSE 'Fisik' END as media_type, \
                COUNT(DISTINCT b.biblio_id) as biblio_count, \
                COUNT(i.item_id) as item_count \
             FROM biblio b \
             LEFT JOIN mst_gmd g ON b.gmd_id = g.gmd_id \
             LEFT JOIN item i ON i.biblio_id = b.biblio_id \
             GROUP BY media_type \
             ORDER BY biblio_count DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&rows)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 5. Top 10 Buku Terpopuler ───────────────────────────────────────────

    /// Daftar buku terpopuler berdasarkan jumlah peminjaman dalam periode tertentu.
    /// Cocok untuk horizontal bar chart.
    #[tool(
        description = "Top N most borrowed books by loan count within a date range. limit: default 10, max 50. Suitable for horizontal bar chart."
    )]
    async fn library_stats_top_books(
        &self,
        Parameters(input): Parameters<TopBooksInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();
        let default_start = (today - chrono::Duration::days(30)).to_string();
        let default_end = today.to_string();
        let start = input
            .start_date
            .as_deref()
            .unwrap_or(&default_start)
            .to_owned();
        let end = input.end_date.as_deref().unwrap_or(&default_end).to_owned();
        let limit = input.limit.unwrap_or(10).min(50) as i64;
        let start_date = chrono::NaiveDate::parse_from_str(&start, "%Y-%m-%d")
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let end_date = chrono::NaiveDate::parse_from_str(&end, "%Y-%m-%d")
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let rows = analytics::top_books(&self.pool, start_date, end_date, limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct TopBooksReport {
            period: String,
            data: Vec<analytics::TopBook>,
        }

        serde_json::to_string_pretty(&TopBooksReport {
            period: format!("{} s/d {}", start, end),
            data: rows,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 6. Peminjaman berdasarkan Program Studi/Unit Kerja ─────────────────

    /// Distribusi peminjaman berdasarkan tipe keanggotaan (Program Studi / Unit Kerja).
    /// Cocok untuk pie chart atau bar chart komposisi peminjam.
    #[tool(
        description = "Loan distribution by membership type (representing faculty/department/study program) within a date range. Suitable for pie or bar chart. group_by is ignored for this tool."
    )]
    async fn library_stats_loans_by_member_type(
        &self,
        Parameters(input): Parameters<StatsPeriodInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();
        let default_start = (today - chrono::Duration::days(30)).to_string();
        let default_end = today.to_string();
        let start = input
            .start_date
            .as_deref()
            .unwrap_or(&default_start)
            .to_owned();
        let end = input.end_date.as_deref().unwrap_or(&default_end).to_owned();

        let rows = sqlx::query_as::<_, MemberTypeLoansRow>(
            "SELECT COALESCE(mt.member_type_name, 'Tidak Diketahui') as member_type, \
                COUNT(l.loan_id) as loan_count \
             FROM loan l \
             JOIN member m ON l.member_id = m.member_id \
             LEFT JOIN mst_member_type mt ON m.member_type_id = mt.member_type_id \
             WHERE DATE(l.loan_date) BETWEEN ? AND ? \
             GROUP BY mt.member_type_id, mt.member_type_name \
             ORDER BY loan_count DESC",
        )
        .bind(&start)
        .bind(&end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct LoansByTypeReport {
            period: String,
            data: Vec<MemberTypeLoansRow>,
        }

        serde_json::to_string_pretty(&LoansByTypeReport {
            period: format!("{} s/d {}", start, end),
            data: rows,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 7. Pertumbuhan Anggota Baru per Periode ─────────────────────────────

    /// Data pertumbuhan anggota baru per bulan atau tahun.
    /// Cocok untuk bar chart atau area chart pertumbuhan anggota.
    #[tool(
        description = "New member growth trend grouped by month or year. group_by: \"month\" (default) | \"year\". Suitable for bar or area chart."
    )]
    async fn library_stats_member_growth(
        &self,
        Parameters(input): Parameters<StatsPeriodInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();
        let default_start = (today - chrono::Duration::days(365)).to_string();
        let default_end = today.to_string();
        let start = input
            .start_date
            .as_deref()
            .unwrap_or(&default_start)
            .to_owned();
        let end = input.end_date.as_deref().unwrap_or(&default_end).to_owned();
        let group_by = input.group_by.as_deref().unwrap_or("month");
        let fmt = if group_by == "year" { "%Y" } else { "%Y-%m" };

        let sql = format!(
            "SELECT DATE_FORMAT(COALESCE(register_date, member_since_date, input_date), '{fmt}') as period, \
                COUNT(*) as new_members \
             FROM member \
             WHERE COALESCE(register_date, member_since_date, input_date) BETWEEN ? AND ? \
             GROUP BY period \
             ORDER BY period",
            fmt = fmt
        );

        let rows = sqlx::query_as::<_, NewMemberRow>(&sql)
            .bind(&start)
            .bind(&end)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let total_new: i64 = rows.iter().map(|r| r.new_members).sum();

        #[derive(Serialize)]
        struct MemberGrowthReport {
            period: String,
            group_by: String,
            total_new_members: i64,
            trend: Vec<NewMemberRow>,
        }

        serde_json::to_string_pretty(&MemberGrowthReport {
            period: format!("{} s/d {}", start, end),
            group_by: group_by.to_owned(),
            total_new_members: total_new,
            trend: rows,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 8. Status Pengadaan: Pembelian vs Hibah ─────────────────────────────

    /// Distribusi sumber pengadaan koleksi (Pembelian vs Hibah vs lain-lain).
    /// Berdasarkan field `source` di tabel item: 0=Pembelian, 1=Hibah.
    /// Cocok untuk stacked bar chart atau pie chart pengadaan.
    #[tool(
        description = "Collection acquisition source breakdown (Purchase vs Donation vs Other) based on item.source field for items added in the selected date range. Suitable for stacked bar or pie chart."
    )]
    async fn library_stats_acquisition_by_source(
        &self,
        Parameters(input): Parameters<StatsPeriodInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();
        let default_start = (today - chrono::Duration::days(30)).to_string();
        let default_end = today.to_string();
        let start = input
            .start_date
            .as_deref()
            .unwrap_or(&default_start)
            .to_owned();
        let end = input.end_date.as_deref().unwrap_or(&default_end).to_owned();

        let rows = sqlx::query_as::<_, AcquisitionSourceRow>(
            "SELECT \
                CASE `source` \
                    WHEN 0 THEN 'Pembelian' \
                    WHEN 1 THEN 'Hibah/Hadiah' \
                    ELSE 'Lain-lain' \
                END as source_label, \
                COUNT(*) as item_count, \
                COUNT(DISTINCT biblio_id) as biblio_count, \
                CAST(COALESCE(SUM(price), 0) AS SIGNED) as total_value \
             FROM item \
             WHERE DATE(input_date) BETWEEN ? AND ? \
             GROUP BY `source` \
             ORDER BY item_count DESC",
        )
        .bind(&start)
        .bind(&end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct AcqReport {
            period: String,
            data: Vec<AcquisitionSourceRow>,
        }

        serde_json::to_string_pretty(&AcqReport {
            period: format!("{} s/d {}", start, end),
            data: rows,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 9. Koleksi Tidur / Dead Wood ────────────────────────────────────────

    /// Daftar koleksi yang tidak pernah dipinjam dalam N tahun terakhir (dead wood).
    /// Berguna untuk evaluasi & deseleksi koleksi.
    #[tool(
        description = "Dead stock / dead wood report: books never borrowed or with no loans in the last N years. years_inactive: default 3. limit: default 50, max 200."
    )]
    async fn library_stats_deadstock(
        &self,
        Parameters(input): Parameters<DeadstockInput>,
    ) -> Result<String, McpError> {
        let years = input.years_inactive.unwrap_or(3).max(1) as i64;
        let limit = input.limit.unwrap_or(50).min(200) as i64;

        let rows = sqlx::query_as::<_, DeadstockRow>(
            "SELECT b.biblio_id, b.title, g.gmd_name, b.classification, \
                COUNT(DISTINCT i.item_id) as item_count, \
                MAX(DATE_FORMAT(l.loan_date, '%Y-%m-%d')) as last_loan_date \
             FROM biblio b \
             LEFT JOIN mst_gmd g ON b.gmd_id = g.gmd_id \
             JOIN item i ON i.biblio_id = b.biblio_id \
             LEFT JOIN loan l ON l.item_code = i.item_code \
             GROUP BY b.biblio_id, b.title, g.gmd_name, b.classification \
             HAVING last_loan_date IS NULL \
                OR last_loan_date < DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL ? YEAR), '%Y-%m-%d') \
             ORDER BY last_loan_date ASC \
             LIMIT ?",
        )
        .bind(years)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct DeadstockReport {
            years_inactive: i64,
            total_found: usize,
            items: Vec<DeadstockRow>,
        }

        serde_json::to_string_pretty(&DeadstockReport {
            years_inactive: years,
            total_found: rows.len(),
            items: rows,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 10. Cost-per-Use Analysis ───────────────────────────────────────────

    /// Analisis ROI (Return on Investment) per judul buku: harga beli dibagi
    /// jumlah peminjaman dalam periode. Nilai rendah = lebih cost-effective.
    /// Hanya item yang memiliki data harga yang disertakan.
    #[tool(
        description = "Cost-per-use ROI analysis per book: item price divided by loan count in selected period. Items with price=0 or NULL are excluded. min_price filter (default 1). Suitable for scatter or bar chart."
    )]
    async fn library_stats_cost_per_use(
        &self,
        Parameters(input): Parameters<CostPerUseInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();
        let default_start = (today - chrono::Duration::days(30)).to_string();
        let default_end = today.to_string();
        let start = input
            .start_date
            .as_deref()
            .unwrap_or(&default_start)
            .to_owned();
        let end = input.end_date.as_deref().unwrap_or(&default_end).to_owned();
        let min_price = input.min_price.unwrap_or(1).max(0);
        let limit = input.limit.unwrap_or(50).min(200) as i64;

        let rows = sqlx::query_as::<_, CostPerUseRow>(
            "SELECT b.biblio_id, b.title, g.gmd_name, \
                CAST(SUM(COALESCE(i.price, 0)) AS SIGNED) as total_price, \
                COUNT(DISTINCT i.item_id) as item_count, \
                COUNT(l.loan_id) as loan_count, \
                CASE WHEN COUNT(l.loan_id) > 0 \
                     THEN CAST(SUM(COALESCE(i.price, 0)) / COUNT(l.loan_id) AS SIGNED) \
                     ELSE NULL END as cost_per_use \
             FROM biblio b \
             LEFT JOIN mst_gmd g ON b.gmd_id = g.gmd_id \
             JOIN item i ON i.biblio_id = b.biblio_id \
             LEFT JOIN loan l ON l.item_code = i.item_code \
                AND DATE(l.loan_date) BETWEEN ? AND ? \
             WHERE i.price IS NOT NULL AND i.price >= ? \
             GROUP BY b.biblio_id, b.title, g.gmd_name \
             ORDER BY cost_per_use DESC \
             LIMIT ?",
        )
        .bind(&start)
        .bind(&end)
        .bind(min_price)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct CostPerUseReport {
            period: String,
            note: &'static str,
            data: Vec<CostPerUseRow>,
        }

        serde_json::to_string_pretty(&CostPerUseReport {
            period: format!("{} s/d {}", start, end),
            note: "cost_per_use = total_price / loan_count in period (NULL = not borrowed)",
            data: rows,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 11. Heatmap Kepadatan Jam Kunjung ───────────────────────────────────

    /// Data heatmap kunjungan per jam (0-23) dan hari dalam seminggu (1=Minggu–7=Sabtu).
    /// Cocok untuk punch card chart atau heatmap.
    #[tool(
        description = "Visitor heatmap data: visit counts by hour of day (0–23) and day of week (1=Sunday–7=Saturday). Suitable for punch card or heatmap chart."
    )]
    async fn library_stats_visitor_heatmap(
        &self,
        Parameters(input): Parameters<HeatmapInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();
        let default_start = (today - chrono::Duration::days(30)).to_string();
        let default_end = today.to_string();
        let start = input
            .start_date
            .as_deref()
            .unwrap_or(&default_start)
            .to_owned();
        let end = input.end_date.as_deref().unwrap_or(&default_end).to_owned();

        let rows = sqlx::query_as::<_, HeatmapRow>(
            "SELECT \
                CAST(HOUR(checkin_date) AS SIGNED) as hour_of_day, \
                CAST(DAYOFWEEK(checkin_date) AS SIGNED) as day_of_week, \
                COUNT(*) as visit_count \
             FROM visitor_count \
             WHERE DATE(checkin_date) BETWEEN ? AND ? \
             GROUP BY hour_of_day, day_of_week \
             ORDER BY day_of_week, hour_of_day",
        )
        .bind(&start)
        .bind(&end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct HeatmapReport {
            period: String,
            note: &'static str,
            data: Vec<HeatmapRow>,
        }

        serde_json::to_string_pretty(&HeatmapReport {
            period: format!("{} s/d {}", start, end),
            note: "day_of_week: 1=Sunday, 2=Monday, ..., 7=Saturday",
            data: rows,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 12. Retensi Anggota (Cohort Analysis) ───────────────────────────────

    /// Analisis retensi anggota: jumlah anggota yang mendaftar per bulan (kohort)
    /// dan berapa yang masih aktif saat ini. Berguna untuk cohort analysis.
    #[tool(
        description = "Member retention cohort analysis: members registered per month vs how many are still active today. cohort_year: start year (default: 2 years ago). months: number of months shown (default 24, max 60)."
    )]
    async fn library_stats_member_retention(
        &self,
        Parameters(input): Parameters<RetentionInput>,
    ) -> Result<String, McpError> {
        let current_year = chrono::Utc::now().date_naive().year();
        let cohort_year = input.cohort_year.unwrap_or(current_year - 2);
        let months = input.months.unwrap_or(24).min(60) as i64;

        let rows = sqlx::query_as::<_, RetentionRow>(
            "SELECT \
                DATE_FORMAT(COALESCE(register_date, member_since_date, input_date), '%Y-%m') as cohort_month, \
                COUNT(*) as registered, \
                COUNT(CASE WHEN expire_date >= CURDATE() AND is_pending = 0 THEN 1 END) as active \
             FROM member \
             WHERE YEAR(COALESCE(register_date, member_since_date, input_date)) >= ? \
             GROUP BY cohort_month \
             ORDER BY cohort_month \
             LIMIT ?",
        )
        .bind(cohort_year)
        .bind(months)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct RetentionResult {
            cohort_month: String,
            registered: i64,
            active: i64,
            retention_rate_pct: f64,
        }

        let data: Vec<RetentionResult> = rows
            .into_iter()
            .map(|r| {
                let rate = if r.registered > 0 {
                    (r.active as f64 / r.registered as f64 * 100.0 * 10.0).round() / 10.0
                } else {
                    0.0
                };
                RetentionResult {
                    cohort_month: r.cohort_month,
                    registered: r.registered,
                    active: r.active,
                    retention_rate_pct: rate,
                }
            })
            .collect();

        #[derive(Serialize)]
        struct RetentionReport {
            cohort_start_year: i32,
            note: &'static str,
            data: Vec<RetentionResult>,
        }

        serde_json::to_string_pretty(&RetentionReport {
            cohort_start_year: cohort_year,
            note: "retention_rate_pct = active / registered * 100 (as of today)",
            data,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    // ─── 13. Kondisi Fisik Koleksi ───────────────────────────────────────────

    /// Distribusi kondisi fisik koleksi berdasarkan item_status.
    /// NULL item_status_id = Baik/Normal; R=Repair; NL=No Loan; MIS=Missing.
    /// Cocok untuk pie chart atau donut chart kondisi koleksi.
    #[tool(
        description = "Physical condition of collection items, grouped by item status. NULL status = Good/Normal, R = Repair, NL = No Loan, MIS = Missing. Suitable for pie or donut chart."
    )]
    async fn library_stats_item_condition(&self) -> Result<String, McpError> {
        let rows = sqlx::query_as::<_, ItemConditionRow>(
            "SELECT \
                COALESCE(s.item_status_name, 'Baik/Normal') as condition_label, \
                COUNT(*) as item_count \
             FROM item \
             LEFT JOIN mst_item_status s ON item.item_status_id = s.item_status_id \
             GROUP BY item.item_status_id, s.item_status_name \
             ORDER BY item_count DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let total_items: i64 = rows.iter().map(|r| r.item_count).sum();

        #[derive(Serialize)]
        struct ConditionResult {
            condition_label: String,
            item_count: i64,
            percentage: f64,
        }
        #[derive(Serialize)]
        struct ConditionReport {
            total_items: i64,
            data: Vec<ConditionResult>,
        }

        let data: Vec<ConditionResult> = rows
            .into_iter()
            .map(|r| {
                let pct = if total_items > 0 {
                    (r.item_count as f64 / total_items as f64 * 1000.0).round() / 10.0
                } else {
                    0.0
                };
                ConditionResult {
                    condition_label: r.condition_label,
                    item_count: r.item_count,
                    percentage: pct,
                }
            })
            .collect();

        serde_json::to_string_pretty(&ConditionReport { total_items, data })
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn date_format_for(group_by: &str) -> &'static str {
    match group_by {
        "week" => "%Y-%u",
        "month" => "%Y-%m",
        _ => "%Y-%m-%d",
    }
}
