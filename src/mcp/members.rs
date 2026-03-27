use rmcp::{ErrorData as McpError, handler::server::wrapper::Parameters, tool, tool_router};
use serde::Serialize;

use super::{LibraryMcpServer, types::*};

#[tool_router(router = members_tool_router, vis = "pub(crate)")]
impl LibraryMcpServer {
    /// Cari anggota perpustakaan berdasarkan nama, ID, atau email.
    #[tool(description = "Cari anggota perpustakaan berdasarkan nama, ID anggota, atau email")]
    async fn search_members(
        &self,
        Parameters(input): Parameters<SearchMembersInput>,
    ) -> Result<String, McpError> {
        let limit = input.limit.unwrap_or(20).min(50) as i64;

        let mut conds = vec!["1=1".to_string()];
        if input.name.is_some() {
            conds.push("member_name LIKE ?".to_string());
        }
        if input.member_id.is_some() {
            conds.push("member_id = ?".to_string());
        }
        if input.email.is_some() {
            conds.push("member_email = ?".to_string());
        }

        let sql = format!(
            "SELECT member_id, member_name, member_email, member_type_id, expire_date, is_pending \
             FROM member WHERE {} ORDER BY member_name LIMIT ?",
            conds.join(" AND ")
        );

        let mut q = sqlx::query_as::<_, MemberRow>(&sql);
        if let Some(ref name) = input.name {
            q = q.bind(format!("%{}%", name));
        }
        if let Some(ref mid) = input.member_id {
            q = q.bind(mid);
        }
        if let Some(ref email) = input.email {
            q = q.bind(email);
        }
        q = q.bind(limit);

        let members = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&members)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Ambil detail anggota termasuk tipe keanggotaan dan batas pinjam.
    #[tool(description = "Ambil detail anggota perpustakaan termasuk tipe keanggotaan, batas pinjam, dan jumlah pinjaman aktif")]
    async fn get_member(
        &self,
        Parameters(input): Parameters<GetMemberInput>,
    ) -> Result<String, McpError> {
        let member = sqlx::query_as::<_, MemberRow>(
            "SELECT member_id, member_name, member_email, member_type_id, expire_date, is_pending \
             FROM member WHERE member_id = ?",
        )
        .bind(&input.member_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| McpError::resource_not_found("Anggota tidak ditemukan", None))?;

        let member_type = if let Some(type_id) = member.member_type_id {
            sqlx::query_as::<_, MemberTypeRow>(
                "SELECT member_type_id, member_type_name, loan_limit, loan_periode \
                 FROM mst_member_type WHERE member_type_id = ?",
            )
            .bind(type_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
        } else {
            None
        };

        let active_loans: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM loan WHERE member_id = ? AND is_return = 0")
                .bind(&member.member_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct MemberDetail {
            member_id: String,
            member_name: String,
            member_email: Option<String>,
            expire_date: chrono::NaiveDate,
            is_pending: i16,
            member_type: Option<MemberTypeRow>,
            active_loans: i64,
        }

        let result = MemberDetail {
            member_id: member.member_id,
            member_name: member.member_name,
            member_email: member.member_email,
            expire_date: member.expire_date,
            is_pending: member.is_pending,
            member_type,
            active_loans,
        };

        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}
