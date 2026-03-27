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
