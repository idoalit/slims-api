use rmcp::{ErrorData as McpError, handler::server::wrapper::Parameters, tool, tool_router};
use serde::Serialize;

use super::{LibraryMcpServer, types::*};

#[tool_router(router = items_tool_router, vis = "pub(crate)")]
impl LibraryMcpServer {
    /// Daftar eksemplar fisik di perpustakaan dengan filter opsional
    /// berdasarkan bibliografi, lokasi, atau ketersediaan.
    #[tool(description = "Daftar eksemplar fisik buku dengan filter opsional berdasarkan bibliografi, lokasi, atau ketersediaan")]
    async fn library_items_list(
        &self,
        Parameters(input): Parameters<ListItemsInput>,
    ) -> Result<String, McpError> {
        let limit = input.limit.unwrap_or(20).min(50) as i64;

        let available_condition =
            "NOT EXISTS (SELECT 1 FROM loan l WHERE l.item_code = i.item_code AND l.is_return = 0)";

        let mut sql = String::from(
            "SELECT i.item_id, i.item_code, i.biblio_id, i.call_number, \
             i.coll_type_id, i.location_id, i.item_status_id FROM item i WHERE ",
        );

        let mut conds = vec!["1=1".to_string()];
        if input.biblio_id.is_some() {
            conds.push("i.biblio_id = ?".to_string());
        }
        if input.location_id.is_some() {
            conds.push("i.location_id = ?".to_string());
        }
        if input.available_only.unwrap_or(false) {
            conds.push(available_condition.to_string());
        }

        sql.push_str(&conds.join(" AND "));
        sql.push_str(" ORDER BY i.item_id DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, ItemRow>(&sql);
        if let Some(bid) = input.biblio_id {
            q = q.bind(bid);
        }
        if let Some(ref loc) = input.location_id {
            q = q.bind(loc);
        }
        q = q.bind(limit);

        let items = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct ItemResult {
            item_id: i64,
            item_code: Option<String>,
            biblio_id: Option<i32>,
            biblio_title: Option<String>,
            call_number: Option<String>,
            location_id: Option<String>,
            item_status_id: Option<String>,
            is_available: bool,
        }

        let mut results = Vec::with_capacity(items.len());
        for item in items {
            let biblio_title: Option<String> = if let Some(bid) = item.biblio_id {
                sqlx::query_scalar("SELECT title FROM biblio WHERE biblio_id = ?")
                    .bind(bid)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            } else {
                None
            };

            let is_available = if let Some(ref code) = item.item_code {
                let on_loan: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM loan WHERE item_code = ? AND is_return = 0")
                        .bind(code)
                        .fetch_one(&self.pool)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                on_loan == 0
            } else {
                false
            };

            results.push(ItemResult {
                item_id: item.item_id,
                item_code: item.item_code,
                biblio_id: item.biblio_id,
                biblio_title,
                call_number: item.call_number,
                location_id: item.location_id,
                item_status_id: item.item_status_id,
                is_available,
            });
        }

        serde_json::to_string_pretty(&results)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}
