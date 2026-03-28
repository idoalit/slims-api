use rmcp::schemars;
use serde::{Deserialize, Serialize};

// ─── Tool input types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchCatalogInput {
    /// Kata kunci pencarian (judul, penulis, topik, atau ISBN)
    pub query: String,
    /// Jumlah hasil maksimal (default: 20, max: 50)
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBookInput {
    /// ID bibliografi buku
    pub biblio_id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListItemsInput {
    /// Filter berdasarkan ID bibliografi
    pub biblio_id: Option<i64>,
    /// Filter berdasarkan ID lokasi
    pub location_id: Option<String>,
    /// Hanya tampilkan item yang tersedia (tidak sedang dipinjam)
    pub available_only: Option<bool>,
    /// Jumlah hasil maksimal (default: 20, max: 50)
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchMembersInput {
    /// Nama anggota (pencarian parsial)
    pub name: Option<String>,
    /// ID anggota (tepat)
    pub member_id: Option<String>,
    /// Email anggota
    pub email: Option<String>,
    /// Jumlah hasil maksimal (default: 20, max: 50)
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetMemberInput {
    /// ID anggota
    pub member_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListLoansInput {
    /// Filter berdasarkan ID anggota
    pub member_id: Option<String>,
    /// Filter berdasarkan kode item
    pub item_code: Option<String>,
    /// Hanya tampilkan peminjaman aktif (belum dikembalikan)
    pub active_only: Option<bool>,
    /// Jumlah hasil maksimal (default: 20, max: 50)
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckoutInput {
    /// Kode item yang akan dipinjam
    pub item_code: String,
    /// ID anggota yang meminjam
    pub member_id: String,
    /// Tanggal jatuh tempo (format: YYYY-MM-DD). Jika tidak diisi, dihitung
    /// dari loan_periode keanggotaan atau default 14 hari.
    pub due_date: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReturnBookInput {
    /// ID peminjaman yang akan dikembalikan
    pub loan_id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetLookupsInput {
    /// Tipe data referensi yang diinginkan.
    /// Nilai yang valid: member_types, locations, coll_types, languages,
    /// gmds, item_statuses, publishers, topics
    pub lookup_type: String,
}

// ─── Internal query result types ─────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct CatalogRow {
    pub biblio_id: i64,
    pub title: String,
    pub call_number: Option<String>,
    pub publish_year: Option<String>,
    pub classification: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct AuthorRow {
    pub author_name: String,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct TopicRow {
    pub topic: String,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct ItemRow {
    pub item_id: i64,
    pub item_code: Option<String>,
    pub biblio_id: Option<i32>,
    pub call_number: Option<String>,
    pub coll_type_id: Option<i32>,
    pub location_id: Option<String>,
    pub item_status_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct LoanRow {
    pub loan_id: i64,
    pub item_code: Option<String>,
    pub member_id: Option<String>,
    pub loan_date: chrono::NaiveDate,
    pub due_date: chrono::NaiveDate,
    pub return_date: Option<chrono::NaiveDate>,
    pub is_return: i32,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct MemberRow {
    pub member_id: String,
    pub member_name: String,
    pub member_email: Option<String>,
    pub member_type_id: Option<i32>,
    pub expire_date: chrono::NaiveDate,
    pub is_pending: i16,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct MemberTypeRow {
    pub member_type_id: i64,
    pub member_type_name: String,
    pub loan_limit: i64,
    pub loan_periode: i64,
}

// ─── Report input types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CirculationReportInput {
    /// Tanggal mulai periode (format: YYYY-MM-DD). Default: 30 hari lalu.
    pub start_date: Option<String>,
    /// Tanggal akhir periode (format: YYYY-MM-DD). Default: hari ini.
    pub end_date: Option<String>,
    /// Jumlah buku terpinjam terbanyak yang ditampilkan (default: 10, max: 50)
    pub top_n: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OverdueReportInput {
    /// Tanggal mulai periode jatuh tempo (format: YYYY-MM-DD). Default: 30 hari lalu.
    pub start_date: Option<String>,
    /// Tanggal akhir periode jatuh tempo (format: YYYY-MM-DD). Default: hari ini.
    pub end_date: Option<String>,
    /// Filter berdasarkan ID anggota (opsional)
    pub member_id: Option<String>,
    /// Filter berdasarkan ID lokasi item (opsional)
    pub location_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CollectionReportInput {
    /// Tanggal mulai periode input data (format: YYYY-MM-DD). Default: 30 hari lalu.
    pub start_date: Option<String>,
    /// Tanggal akhir periode input data (format: YYYY-MM-DD). Default: hari ini.
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NewCollectionReportInput {
    /// Tanggal mulai periode (format: YYYY-MM-DD). Default: 30 hari lalu.
    pub start_date: Option<String>,
    /// Tanggal akhir periode (format: YYYY-MM-DD). Default: hari ini.
    pub end_date: Option<String>,
    /// Jumlah judul baru yang ditampilkan dalam daftar (default: 20, max: 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MemberReportInput {
    /// Tanggal mulai periode (format: YYYY-MM-DD). Default: 30 hari lalu.
    pub start_date: Option<String>,
    /// Tanggal akhir periode (format: YYYY-MM-DD). Default: hari ini.
    pub end_date: Option<String>,
    /// Filter berdasarkan ID tipe anggota (opsional)
    pub member_type_id: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct VisitorReportInput {
    /// Tanggal mulai periode (format: YYYY-MM-DD). Default: 30 hari lalu.
    pub start_date: Option<String>,
    /// Tanggal akhir periode (format: YYYY-MM-DD). Default: hari ini.
    pub end_date: Option<String>,
    /// Pengelompokan detail: "day" (per hari, default) atau "month" (per bulan)
    pub group_by: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FinesReportInput {
    /// Tanggal mulai periode transaksi denda (format: YYYY-MM-DD). Default: 30 hari lalu.
    pub start_date: Option<String>,
    /// Tanggal akhir periode transaksi denda (format: YYYY-MM-DD). Default: hari ini.
    pub end_date: Option<String>,
    /// Filter berdasarkan ID anggota (opsional)
    pub member_id: Option<String>,
    /// Hanya tampilkan anggota dengan denda yang belum lunas (default: true)
    pub outstanding_only: Option<bool>,
}

// ─── Report row types ─────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct CirculationSummaryRow {
    pub total_loans: i64,
    pub returned: i64,
    pub active: i64,
    pub overdue: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct TopBorrowedRow {
    pub biblio_id: i64,
    pub title: String,
    pub loan_count: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct OverdueRow {
    pub loan_id: i64,
    pub item_code: Option<String>,
    pub member_id: Option<String>,
    pub member_name: String,
    pub loan_date: String,
    pub due_date: String,
    pub days_overdue: i64,
    pub estimated_fine: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct CollectionByGmdRow {
    pub gmd_name: String,
    pub biblio_count: i64,
    pub item_count: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct CollectionByLocationRow {
    pub location_name: String,
    pub item_count: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct CollectionByCollTypeRow {
    pub coll_type_name: String,
    pub item_count: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct MemberStatRow {
    pub member_type_name: String,
    pub total: i64,
    pub active: i64,
    pub pending: i64,
    pub expired: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct TopBorrowerRow {
    pub member_id: String,
    pub member_name: String,
    pub active_loans: i64,
    pub total_loans: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct VisitorSummaryRow {
    pub total_visits: i64,
    pub unique_members: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct VisitorDayRow {
    pub visit_date: String,
    pub visitor_count: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct FinesRow {
    pub member_id: String,
    pub member_name: String,
    pub total_debet: i64,
    pub total_credit: i64,
    pub outstanding: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct NewCollectionByGmdRow {
    pub gmd_name: String,
    pub new_biblio: i64,
    pub new_items: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct NewCollectionByLocationRow {
    pub location_name: String,
    pub new_items: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(super) struct NewBiblioRow {
    pub biblio_id: i64,
    pub title: String,
    pub gmd_name: Option<String>,
    pub classification: Option<String>,
    pub call_number: Option<String>,
    pub item_count: i64,
    pub input_date: Option<String>,
}
