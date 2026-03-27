use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool_handler,
};
use sqlx::MySqlPool;

mod catalog;
mod items;
mod loans;
mod lookups;
mod members;
pub mod types;

// ─── Server struct ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct LibraryMcpServer {
    pub pool: MySqlPool,
    tool_router: ToolRouter<Self>,
}

impl LibraryMcpServer {
    pub fn new(pool: MySqlPool) -> Self {
        let mut router = Self::catalog_tool_router();
        router.merge(Self::items_tool_router());
        router.merge(Self::members_tool_router());
        router.merge(Self::loans_tool_router());
        router.merge(Self::lookups_tool_router());
        Self { pool, tool_router: router }
    }
}

// ─── ServerHandler implementation ─────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for LibraryMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::new(
            "slims-library-mcp",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "MCP server untuk Sistem Perpustakaan SLiMS. \
             Tersedia tools untuk: mencari & membaca katalog buku, melihat eksemplar, \
             mengelola anggota, membuat & mengembalikan peminjaman, serta membaca data referensi.",
        )
    }
}

