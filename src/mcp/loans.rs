use rmcp::{ErrorData as McpError, handler::server::wrapper::Parameters, tool, tool_router};
use serde::Serialize;

use super::{LibraryMcpServer, types::*};

#[tool_router(router = loans_tool_router, vis = "pub(crate)")]
impl LibraryMcpServer {
    /// Daftar peminjaman dengan filter opsional berdasarkan anggota,
    /// kode item, atau status pengembalian.
    #[tool(description = "List loans with optional filters by member ID, item code, or active status")]
    async fn library_loans_list(
        &self,
        Parameters(input): Parameters<ListLoansInput>,
    ) -> Result<String, McpError> {
        let limit = input.limit.unwrap_or(20).min(50) as i64;

        let mut conds = vec!["1=1".to_string()];
        if input.member_id.is_some() {
            conds.push("l.member_id = ?".to_string());
        }
        if input.item_code.is_some() {
            conds.push("l.item_code = ?".to_string());
        }
        if input.active_only.unwrap_or(false) {
            conds.push("l.is_return = 0".to_string());
        }

        let sql = format!(
            "SELECT loan_id, item_code, member_id, loan_date, due_date, return_date, is_return \
             FROM loan l WHERE {} ORDER BY l.loan_date DESC LIMIT ?",
            conds.join(" AND ")
        );

        let mut q = sqlx::query_as::<_, LoanRow>(&sql);
        if let Some(ref mid) = input.member_id {
            q = q.bind(mid);
        }
        if let Some(ref code) = input.item_code {
            q = q.bind(code);
        }
        q = q.bind(limit);

        let loans = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&loans)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Buat peminjaman baru (checkout buku). Pastikan item tersedia dan
    /// anggota masih aktif sebelum memanggil tool ini.
    #[tool(description = "Create a new checkout transaction using item code and member ID. Due date is calculated from membership type when omitted.")]
    async fn library_loans_checkout_create(
        &self,
        Parameters(input): Parameters<CheckoutInput>,
    ) -> Result<String, McpError> {
        let item_code = &input.item_code;
        let item_exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM item WHERE item_code = ?")
                .bind(item_code)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if item_exists == 0 {
            return Err(McpError::invalid_params(
                format!("Item dengan kode '{}' tidak ditemukan", item_code),
                None,
            ));
        }

        let on_loan: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM loan WHERE item_code = ? AND is_return = 0")
                .bind(item_code)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if on_loan > 0 {
            return Err(McpError::invalid_params(
                format!("Item '{}' sedang dipinjam", item_code),
                None,
            ));
        }

        let member_id = &input.member_id;
        let member = sqlx::query_as::<_, MemberRow>(
            "SELECT member_id, member_name, member_email, member_type_id, expire_date, is_pending \
             FROM member WHERE member_id = ?",
        )
        .bind(member_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| {
            McpError::invalid_params(
                format!("Anggota '{}' tidak ditemukan", member_id),
                None,
            )
        })?;

        if member.is_pending != 0 {
            return Err(McpError::invalid_params(
                "Keanggotaan sedang pending, tidak dapat meminjam".to_string(),
                None,
            ));
        }

        let today = chrono::Utc::now().date_naive();
        let due_date = if let Some(ref d) = input.due_date {
            d.parse::<chrono::NaiveDate>().map_err(|_| {
                McpError::invalid_params(
                    "Format tanggal tidak valid, gunakan YYYY-MM-DD".to_string(),
                    None,
                )
            })?
        } else {
            let loan_periode = if let Some(type_id) = member.member_type_id {
                sqlx::query_scalar::<_, i64>(
                    "SELECT loan_periode FROM mst_member_type WHERE member_type_id = ?",
                )
                .bind(type_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
                .unwrap_or(14)
            } else {
                14
            };
            today + chrono::Duration::days(loan_periode)
        };

        let result = sqlx::query(
            "INSERT INTO loan (item_code, member_id, loan_date, due_date, is_lent, is_return) \
             VALUES (?, ?, ?, ?, 1, 0)",
        )
        .bind(item_code)
        .bind(member_id)
        .bind(today)
        .bind(due_date)
        .execute(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let loan_id = result.last_insert_id() as i64;

        #[derive(Serialize)]
        struct CheckoutResult {
            loan_id: i64,
            item_code: String,
            member_id: String,
            member_name: String,
            loan_date: chrono::NaiveDate,
            due_date: chrono::NaiveDate,
        }

        serde_json::to_string_pretty(&CheckoutResult {
            loan_id,
            item_code: input.item_code,
            member_id: input.member_id,
            member_name: member.member_name,
            loan_date: today,
            due_date,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Kembalikan buku yang dipinjam berdasarkan loan_id.
    #[tool(description = "Register a returned loan by loan_id")]
    async fn library_loans_return_register(
        &self,
        Parameters(input): Parameters<ReturnBookInput>,
    ) -> Result<String, McpError> {
        let today = chrono::Utc::now().date_naive();

        let updated = sqlx::query(
            "UPDATE loan SET return_date = ?, is_return = 1, actual = ? \
             WHERE loan_id = ? AND is_return = 0",
        )
        .bind(today)
        .bind(today)
        .bind(input.loan_id)
        .execute(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if updated.rows_affected() == 0 {
            return Err(McpError::resource_not_found(
                "Peminjaman tidak ditemukan atau sudah dikembalikan",
                None,
            ));
        }

        let loan = sqlx::query_as::<_, LoanRow>(
            "SELECT loan_id, item_code, member_id, loan_date, due_date, return_date, is_return \
             FROM loan WHERE loan_id = ?",
        )
        .bind(input.loan_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&loan)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}
