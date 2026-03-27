use rmcp::{ErrorData as McpError, handler::server::wrapper::Parameters, tool, tool_router};
use serde::Serialize;

use super::{LibraryMcpServer, types::*};

#[tool_router(router = catalog_tool_router, vis = "pub(crate)")]
impl LibraryMcpServer {
    /// Cari buku/bibliografi di katalog perpustakaan berdasarkan
    /// judul, penulis, topik, atau ISBN/ISSN.
    #[tool(description = "Cari buku di katalog perpustakaan berdasarkan judul, penulis, topik, atau ISBN")]
    async fn search_catalog(
        &self,
        Parameters(input): Parameters<SearchCatalogInput>,
    ) -> Result<String, McpError> {
        let limit = input.limit.unwrap_or(20).min(50) as i64;
        let pattern = format!("%{}%", input.query);

        let ids_subquery = r#"
            SELECT biblio_id FROM biblio WHERE title LIKE ?
            UNION
            SELECT ba.biblio_id
            FROM biblio_author ba
            JOIN mst_author a ON a.author_id = ba.author_id
            WHERE a.author_name LIKE ?
            UNION
            SELECT bt.biblio_id
            FROM biblio_topic bt
            JOIN mst_topic t ON t.topic_id = bt.topic_id
            WHERE t.topic LIKE ?
            UNION
            SELECT biblio_id FROM biblio WHERE isbn_issn LIKE ?
        "#;

        let data_sql = format!(
            r#"
            SELECT b.biblio_id, b.title, b.call_number, b.publish_year, b.classification
            FROM biblio b
            JOIN ({ids}) ids ON ids.biblio_id = b.biblio_id
            ORDER BY b.biblio_id DESC
            LIMIT ?
            "#,
            ids = ids_subquery
        );

        let rows = sqlx::query_as::<_, CatalogRow>(&data_sql)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct CatalogResult {
            biblio_id: i64,
            title: String,
            call_number: Option<String>,
            publish_year: Option<String>,
            classification: Option<String>,
            authors: Vec<String>,
        }

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let authors = sqlx::query_as::<_, AuthorRow>(
                "SELECT a.author_name FROM biblio_author ba \
                 JOIN mst_author a ON a.author_id = ba.author_id \
                 WHERE ba.biblio_id = ? LIMIT 5",
            )
            .bind(row.biblio_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            results.push(CatalogResult {
                biblio_id: row.biblio_id,
                title: row.title,
                call_number: row.call_number,
                publish_year: row.publish_year,
                classification: row.classification,
                authors: authors.into_iter().map(|a| a.author_name).collect(),
            });
        }

        serde_json::to_string_pretty(&results)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Ambil detail lengkap sebuah buku berdasarkan biblio_id, termasuk
    /// penulis, topik, dan daftar eksemplar yang tersedia.
    #[tool(description = "Ambil detail lengkap buku berdasarkan biblio_id, termasuk penulis, topik, dan daftar eksemplar")]
    async fn get_book(
        &self,
        Parameters(input): Parameters<GetBookInput>,
    ) -> Result<String, McpError> {
        #[derive(sqlx::FromRow)]
        struct BiblioFull {
            biblio_id: i64,
            title: String,
            classification: Option<String>,
            call_number: Option<String>,
            publish_year: Option<String>,
            isbn_issn: Option<String>,
        }

        let biblio = sqlx::query_as::<_, BiblioFull>(
            "SELECT biblio_id, title, classification, call_number, publish_year, isbn_issn \
             FROM biblio WHERE biblio_id = ?",
        )
        .bind(input.biblio_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| McpError::resource_not_found("Buku tidak ditemukan", None))?;

        let authors = sqlx::query_as::<_, AuthorRow>(
            "SELECT a.author_name FROM biblio_author ba \
             JOIN mst_author a ON a.author_id = ba.author_id \
             WHERE ba.biblio_id = ?",
        )
        .bind(biblio.biblio_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let topics = sqlx::query_as::<_, TopicRow>(
            "SELECT t.topic FROM biblio_topic bt \
             JOIN mst_topic t ON bt.topic_id = t.topic_id \
             WHERE bt.biblio_id = ?",
        )
        .bind(biblio.biblio_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let items = sqlx::query_as::<_, ItemRow>(
            "SELECT item_id, item_code, biblio_id, call_number, coll_type_id, location_id, item_status_id \
             FROM item WHERE biblio_id = ? ORDER BY item_id",
        )
        .bind(biblio.biblio_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let available_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM item i WHERE i.biblio_id = ? \
             AND NOT EXISTS (SELECT 1 FROM loan l WHERE l.item_code = i.item_code AND l.is_return = 0)",
        )
        .bind(biblio.biblio_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct BookDetail {
            biblio_id: i64,
            title: String,
            classification: Option<String>,
            call_number: Option<String>,
            publish_year: Option<String>,
            isbn_issn: Option<String>,
            authors: Vec<String>,
            topics: Vec<String>,
            total_items: usize,
            available_items: i64,
            items: Vec<ItemRow>,
        }

        let result = BookDetail {
            biblio_id: biblio.biblio_id,
            title: biblio.title,
            classification: biblio.classification,
            call_number: biblio.call_number,
            publish_year: biblio.publish_year,
            isbn_issn: biblio.isbn_issn,
            authors: authors.into_iter().map(|a| a.author_name).collect(),
            topics: topics.into_iter().map(|t| t.topic).collect(),
            total_items: items.len(),
            available_items: available_count,
            items,
        };

        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}
