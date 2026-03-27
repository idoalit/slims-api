use rmcp::{ErrorData as McpError, handler::server::wrapper::Parameters, tool, tool_router};
use serde::Serialize;

use super::{LibraryMcpServer, types::*};

#[tool_router(router = reports_tool_router, vis = "pub(crate)")]
impl LibraryMcpServer {
    /// Laporan sirkulasi peminjaman: ringkasan total, dikembalikan, aktif, terlambat,
    /// dan daftar N buku terpinjam terbanyak dalam rentang waktu tertentu.
    #[tool(description = "Laporan sirkulasi perpustakaan: ringkasan total peminjaman, dikembalikan, aktif, terlambat, serta daftar N buku terpinjam terbanyak dalam rentang tanggal tertentu")]
    async fn get_circulation_report(
        &self,
        Parameters(input): Parameters<CirculationReportInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();
        let default_start = (today - chrono::Duration::days(30)).to_string();
        let default_end = today.to_string();
        let start = input.start_date.as_deref().unwrap_or(&default_start).to_owned();
        let end = input.end_date.as_deref().unwrap_or(&default_end).to_owned();
        let top_n = input.top_n.unwrap_or(10).min(50) as i64;

        // Setiap metrik menggunakan kolom tanggal yang berbeda:
        //   total_loans  → loan_date (transaksi peminjaman baru dalam periode)
        //   returned     → return_date (pengembalian yang terjadi dalam periode)
        //   active       → snapshot hari ini: semua pinjaman yang belum kembali
        //   overdue      → snapshot hari ini: pinjaman aktif yang sudah lewat jatuh tempo
        let summary = sqlx::query_as::<_, CirculationSummaryRow>(
            "SELECT \
                (SELECT COUNT(*) FROM loan WHERE loan_date BETWEEN ? AND ?) as total_loans, \
                (SELECT COUNT(*) FROM loan WHERE is_return = 1 AND return_date BETWEEN ? AND ?) as returned, \
                (SELECT COUNT(*) FROM loan WHERE is_return = 0) as active, \
                (SELECT COUNT(*) FROM loan WHERE is_return = 0 AND due_date < CURDATE()) as overdue",
        )
        .bind(&start)
        .bind(&end)
        .bind(&start)
        .bind(&end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let top_borrowed = sqlx::query_as::<_, TopBorrowedRow>(
            "SELECT b.biblio_id, b.title, COUNT(l.loan_id) as loan_count \
             FROM loan l \
             JOIN item i ON l.item_code = i.item_code \
             JOIN biblio b ON i.biblio_id = b.biblio_id \
             WHERE l.loan_date BETWEEN ? AND ? \
             GROUP BY b.biblio_id, b.title \
             ORDER BY loan_count DESC \
             LIMIT ?",
        )
        .bind(&start)
        .bind(&end)
        .bind(top_n)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct CirculationReport {
            period: String,
            summary: CirculationSummaryRow,
            top_borrowed: Vec<TopBorrowedRow>,
        }

        serde_json::to_string_pretty(&CirculationReport {
            period: format!("{} s/d {}", start, end),
            summary,
            top_borrowed,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Laporan keterlambatan pengembalian buku: daftar peminjaman yang melewati
    /// tanggal jatuh tempo beserta estimasi denda.
    #[tool(description = "Laporan buku terlambat dikembalikan: daftar peminjaman melewati jatuh tempo beserta estimasi denda. Filter opsional berdasarkan ID anggota atau ID lokasi item.")]
    async fn get_overdue_report(
        &self,
        Parameters(input): Parameters<OverdueReportInput>,
    ) -> Result<String, McpError> {
        let mut conds = vec![
            "l.is_return = 0".to_string(),
            "l.due_date < CURDATE()".to_string(),
        ];
        if input.member_id.is_some() {
            conds.push("l.member_id = ?".to_string());
        }
        if input.location_id.is_some() {
            conds.push("i.location_id = ?".to_string());
        }

        let sql = format!(
            "SELECT l.loan_id, l.item_code, l.member_id, m.member_name, \
                DATE_FORMAT(l.loan_date, '%Y-%m-%d') as loan_date, \
                DATE_FORMAT(l.due_date, '%Y-%m-%d') as due_date, \
                CAST(DATEDIFF(CURDATE(), l.due_date) AS SIGNED) as days_overdue, \
                CAST(COALESCE(mt.fine_each_day, 0) * DATEDIFF(CURDATE(), l.due_date) AS SIGNED) as estimated_fine \
             FROM loan l \
             JOIN member m ON l.member_id = m.member_id \
             LEFT JOIN mst_member_type mt ON m.member_type_id = mt.member_type_id \
             LEFT JOIN item i ON l.item_code = i.item_code \
             WHERE {} \
             ORDER BY days_overdue DESC \
             LIMIT 100",
            conds.join(" AND ")
        );

        let mut q = sqlx::query_as::<_, OverdueRow>(&sql);
        if let Some(ref mid) = input.member_id {
            q = q.bind(mid);
        }
        if let Some(ref loc) = input.location_id {
            q = q.bind(loc);
        }

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&rows)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Statistik koleksi perpustakaan: total bibliografi dan eksemplar,
    /// dikelompokkan berdasarkan GMD, lokasi, dan tipe koleksi.
    #[tool(description = "Laporan statistik koleksi perpustakaan: total jumlah bibliografi dan eksemplar, dikelompokkan berdasarkan jenis bahan (GMD), lokasi, dan tipe koleksi")]
    async fn get_collection_report(
        &self,
        Parameters(_input): Parameters<CollectionReportInput>,
    ) -> Result<String, McpError> {
        let total_biblio: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM biblio")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let total_items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let by_gmd = sqlx::query_as::<_, CollectionByGmdRow>(
            "SELECT g.gmd_name, \
                COUNT(DISTINCT b.biblio_id) as biblio_count, \
                COUNT(i.item_id) as item_count \
             FROM mst_gmd g \
             LEFT JOIN biblio b ON b.gmd_id = g.gmd_id \
             LEFT JOIN item i ON i.biblio_id = b.biblio_id \
             GROUP BY g.gmd_id, g.gmd_name \
             HAVING biblio_count > 0 \
             ORDER BY biblio_count DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let by_location = sqlx::query_as::<_, CollectionByLocationRow>(
            "SELECT l.location_name, COUNT(i.item_id) as item_count \
             FROM mst_location l \
             LEFT JOIN item i ON i.location_id = l.location_id \
             GROUP BY l.location_id, l.location_name \
             HAVING item_count > 0 \
             ORDER BY item_count DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let by_coll_type = sqlx::query_as::<_, CollectionByCollTypeRow>(
            "SELECT ct.coll_type_name, COUNT(i.item_id) as item_count \
             FROM mst_coll_type ct \
             LEFT JOIN item i ON i.coll_type_id = ct.coll_type_id \
             GROUP BY ct.coll_type_id, ct.coll_type_name \
             HAVING item_count > 0 \
             ORDER BY item_count DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct CollectionReport {
            total_biblio: i64,
            total_items: i64,
            by_gmd: Vec<CollectionByGmdRow>,
            by_location: Vec<CollectionByLocationRow>,
            by_coll_type: Vec<CollectionByCollTypeRow>,
        }

        serde_json::to_string_pretty(&CollectionReport {
            total_biblio,
            total_items,
            by_gmd,
            by_location,
            by_coll_type,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Statistik anggota perpustakaan: breakdown per tipe keanggotaan
    /// (aktif/pending/kedaluwarsa) dan 10 peminjam terbanyak.
    #[tool(description = "Laporan statistik anggota perpustakaan: jumlah anggota per tipe (aktif/pending/kedaluwarsa) dan daftar 10 peminjam terbanyak. Filter opsional berdasarkan tipe keanggotaan.")]
    async fn get_member_report(
        &self,
        Parameters(input): Parameters<MemberReportInput>,
    ) -> Result<String, McpError> {
        let where_clause = if input.member_type_id.is_some() {
            "WHERE m.member_type_id = ?".to_string()
        } else {
            String::new()
        };

        let sql = format!(
            "SELECT COALESCE(mt.member_type_name, 'Tidak Terdaftar') as member_type_name, \
                COUNT(*) as total, \
                COUNT(CASE WHEN m.is_pending = 0 AND m.expire_date >= CURDATE() THEN 1 END) as active, \
                COUNT(CASE WHEN m.is_pending = 1 THEN 1 END) as pending, \
                COUNT(CASE WHEN m.expire_date < CURDATE() AND m.is_pending = 0 THEN 1 END) as expired \
             FROM member m \
             LEFT JOIN mst_member_type mt ON m.member_type_id = mt.member_type_id \
             {} \
             GROUP BY m.member_type_id, mt.member_type_name \
             ORDER BY total DESC",
            where_clause
        );

        let mut q = sqlx::query_as::<_, MemberStatRow>(&sql);
        if let Some(type_id) = input.member_type_id {
            q = q.bind(type_id);
        }

        let by_type = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let top_borrowers = sqlx::query_as::<_, TopBorrowerRow>(
            "SELECT m.member_id, m.member_name, \
                COUNT(CASE WHEN l.is_return = 0 THEN 1 END) as active_loans, \
                COUNT(*) as total_loans \
             FROM member m \
             JOIN loan l ON m.member_id = l.member_id \
             GROUP BY m.member_id, m.member_name \
             ORDER BY total_loans DESC \
             LIMIT 10",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct MemberReport {
            by_type: Vec<MemberStatRow>,
            top_borrowers: Vec<TopBorrowerRow>,
        }

        serde_json::to_string_pretty(&MemberReport { by_type, top_borrowers })
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Laporan kunjungan perpustakaan dari tabel visitor_count.
    /// Detail dapat dikelompokkan per hari atau per bulan.
    #[tool(description = "Laporan kunjungan perpustakaan: total kunjungan, anggota unik, dan detail per hari atau per bulan. Filter berdasarkan rentang tanggal; group_by: \"day\" (default) atau \"month\".")]
    async fn get_visitor_report(
        &self,
        Parameters(input): Parameters<VisitorReportInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();
        let default_start = (today - chrono::Duration::days(30)).to_string();
        let default_end = today.to_string();
        let start = input.start_date.as_deref().unwrap_or(&default_start).to_owned();
        let end = input.end_date.as_deref().unwrap_or(&default_end).to_owned();
        let group_by = input.group_by.as_deref().unwrap_or("day").to_owned();

        let summary = sqlx::query_as::<_, VisitorSummaryRow>(
            "SELECT COUNT(*) as total_visits, COUNT(DISTINCT member_id) as unique_members \
             FROM visitor_count \
             WHERE DATE(checkin_date) BETWEEN ? AND ?",
        )
        .bind(&start)
        .bind(&end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let date_format = if group_by == "month" { "%Y-%m" } else { "%Y-%m-%d" };
        let detail_sql = format!(
            "SELECT DATE_FORMAT(checkin_date, '{fmt}') as visit_date, COUNT(*) as visitor_count \
             FROM visitor_count \
             WHERE DATE(checkin_date) BETWEEN ? AND ? \
             GROUP BY DATE_FORMAT(checkin_date, '{fmt}') \
             ORDER BY visit_date",
            fmt = date_format
        );

        let detail = sqlx::query_as::<_, VisitorDayRow>(&detail_sql)
            .bind(&start)
            .bind(&end)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct VisitorReport {
            period: String,
            group_by: String,
            summary: VisitorSummaryRow,
            detail: Vec<VisitorDayRow>,
        }

        serde_json::to_string_pretty(&VisitorReport {
            period: format!("{} s/d {}", start, end),
            group_by,
            summary,
            detail,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Laporan denda anggota: total debet, kredit, dan sisa denda yang
    /// belum dibayar per anggota. Default hanya menampilkan yang memiliki tunggakan.
    #[tool(description = "Laporan denda anggota perpustakaan: total debet, kredit, dan sisa tunggakan per anggota. Filter opsional berdasarkan ID anggota; outstanding_only (default: true) untuk menyaring yang belum lunas.")]
    async fn get_fines_report(
        &self,
        Parameters(input): Parameters<FinesReportInput>,
    ) -> Result<String, McpError> {
        let outstanding_only = input.outstanding_only.unwrap_or(true);

        let where_clause = if input.member_id.is_some() {
            "WHERE f.member_id = ?".to_string()
        } else {
            String::new()
        };
        let having = if outstanding_only { "HAVING outstanding > 0" } else { "" };

        let sql = format!(
            "SELECT f.member_id, m.member_name, \
                CAST(SUM(f.debet) AS SIGNED) as total_debet, \
                CAST(SUM(f.credit) AS SIGNED) as total_credit, \
                CAST(SUM(f.debet) - SUM(f.credit) AS SIGNED) as outstanding \
             FROM fines f \
             JOIN member m ON f.member_id = m.member_id \
             {} \
             GROUP BY f.member_id, m.member_name \
             {} \
             ORDER BY outstanding DESC \
             LIMIT 100",
            where_clause, having
        );

        let mut q = sqlx::query_as::<_, FinesRow>(&sql);
        if let Some(ref mid) = input.member_id {
            q = q.bind(mid);
        }

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&rows)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}
