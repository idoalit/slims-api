use rmcp::{ErrorData as McpError, handler::server::wrapper::Parameters, tool, tool_router};
use serde::Serialize;

use super::{LibraryMcpServer, types::*};

#[tool_router(router = lookups_tool_router, vis = "pub(crate)")]
impl LibraryMcpServer {
    /// Ambil data referensi perpustakaan seperti tipe anggota, lokasi,
    /// jenis koleksi, bahasa, GMD, status item, penerbit, atau topik.
    #[tool(description = "Get library reference data. Lookup types: member_types, locations, coll_types, languages, gmds, item_statuses, publishers, topics")]
    async fn library_lookups_list(
        &self,
        Parameters(input): Parameters<GetLookupsInput>,
    ) -> Result<String, McpError> {
        let json = match input.lookup_type.to_lowercase().as_str() {
            "member_types" => {
                let rows = sqlx::query_as::<_, MemberTypeRow>(
                    "SELECT member_type_id, member_type_name, loan_limit, loan_periode \
                     FROM mst_member_type ORDER BY member_type_name",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&rows)
            }
            "locations" => {
                #[derive(sqlx::FromRow, Serialize)]
                struct Loc {
                    location_id: String,
                    location_name: Option<String>,
                }
                let rows = sqlx::query_as::<_, Loc>(
                    "SELECT location_id, location_name FROM mst_location ORDER BY location_name",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&rows)
            }
            "coll_types" => {
                #[derive(sqlx::FromRow, Serialize)]
                struct CollType {
                    coll_type_id: i64,
                    coll_type_name: String,
                }
                let rows = sqlx::query_as::<_, CollType>(
                    "SELECT coll_type_id, coll_type_name FROM mst_coll_type ORDER BY coll_type_name",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&rows)
            }
            "languages" => {
                #[derive(sqlx::FromRow, Serialize)]
                struct Lang {
                    language_id: String,
                    language_name: String,
                }
                let rows = sqlx::query_as::<_, Lang>(
                    "SELECT language_id, language_name FROM mst_language ORDER BY language_name",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&rows)
            }
            "gmds" => {
                #[derive(sqlx::FromRow, Serialize)]
                struct Gmd {
                    gmd_id: i64,
                    gmd_code: Option<String>,
                    gmd_name: String,
                }
                let rows = sqlx::query_as::<_, Gmd>(
                    "SELECT gmd_id, gmd_code, gmd_name FROM mst_gmd ORDER BY gmd_name",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&rows)
            }
            "item_statuses" => {
                #[derive(sqlx::FromRow, Serialize)]
                struct ItemStatus {
                    item_status_id: String,
                    item_status_name: String,
                    no_loan: i16,
                }
                let rows = sqlx::query_as::<_, ItemStatus>(
                    "SELECT item_status_id, item_status_name, no_loan \
                     FROM mst_item_status ORDER BY item_status_name",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&rows)
            }
            "publishers" => {
                #[derive(sqlx::FromRow, Serialize)]
                struct Pub {
                    publisher_id: i64,
                    publisher_name: String,
                }
                let rows = sqlx::query_as::<_, Pub>(
                    "SELECT publisher_id, publisher_name FROM mst_publisher \
                     ORDER BY publisher_name LIMIT 100",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&rows)
            }
            "topics" => {
                #[derive(sqlx::FromRow, Serialize)]
                struct Topic {
                    topic_id: i64,
                    topic: String,
                    topic_type: String,
                }
                let rows = sqlx::query_as::<_, Topic>(
                    "SELECT topic_id, topic, topic_type FROM mst_topic ORDER BY topic LIMIT 100",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&rows)
            }
            unknown => {
                return Err(McpError::invalid_params(
                    format!(
                        "Tipe lookup '{}' tidak dikenal. Pilih dari: member_types, locations, \
                         coll_types, languages, gmds, item_statuses, publishers, topics",
                        unknown
                    ),
                    None,
                ))
            }
        };

        json.map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}
