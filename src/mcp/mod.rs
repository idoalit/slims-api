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
mod reports;
mod stats;
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
        router.merge(Self::reports_tool_router());
        router.merge(Self::stats_tool_router());
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
            "MCP server for the SLiMS library system. \
             Tools are grouped into six areas: \
             (1) Catalog — search bibliographic records, get full book detail. \
             (2) Items — list item copies with availability filters. \
             (3) Members — search members, get membership detail and loan history. \
             (4) Loans — list loans, create checkouts, register returns. \
             (5) Lookups — reference data: member types, locations, GMDs, collection types, item statuses, publishers, topics. \
             (6) Reports — circulation summary, overdue loans with fines, collection overview, member overview, visitor activity, fines ledger, collection growth. \
             (7) Stats (chart data) — time-series and distribution data ready for charting: \
               circulation trend (loans vs returns by day/week/month), \
               visitor trend (by day/week/month), \
               collection composition by DDC class, \
               physical vs digital media comparison, \
               top N most borrowed books, \
               loan distribution by membership type / study program, \
               new member growth trend, \
               acquisition source breakdown (purchase vs donation), \
               dead-stock / inactive collection analysis, \
               cost-per-use ROI per title, \
               visitor heatmap (hour-of-day × day-of-week), \
               member retention cohort analysis, \
               item physical condition breakdown.",
        )
    }
}

