/// MCP stdio transport binary for SLiMS Library API.
///
/// Digunakan oleh AI agents seperti Claude Desktop, Claude Code, Cursor, dll.
/// via konfigurasi MCP yang menjalankan binary ini sebagai proses terpisah.
///
/// Konfigurasi Claude Desktop (~/Library/Application Support/Claude/claude_desktop_config.json):
/// ```json
/// {
///   "mcpServers": {
///     "slims-library": {
///       "command": "/path/to/slims-rest-api/target/release/mcp_stdio",
///       "env": {
///         "DB_HOST": "localhost",
///         "DB_PORT": "3306",
///         "DB_USER": "root",
///         "DB_PASSWORD": "yourpassword",
///         "DB_NAME": "slims9_bulians"
///       }
///     }
///   }
/// }
/// ```
use dotenvy::dotenv;
use rmcp::ServiceExt;
use sqlx::mysql::MySqlPoolOptions;
use tracing_subscriber::EnvFilter;

#[path = "../analytics.rs"]
mod analytics;
#[path = "../mcp/mod.rs"]
mod mcp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    // Log to stderr so it doesn't interfere with the stdio MCP protocol
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let database_url = build_database_url();

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Gagal terhubung ke database: {}", e))?;

    tracing::info!("SLiMS MCP server starting (stdio)");

    let server = mcp::LibraryMcpServer::new(pool);
    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| tracing::error!("MCP server error: {e}"))?;

    service.waiting().await?;
    Ok(())
}

fn build_database_url() -> String {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        return url;
    }

    let host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".into());
    let port = std::env::var("DB_PORT").unwrap_or_else(|_| "3306".into());
    let user = std::env::var("DB_USER").unwrap_or_else(|_| "root".into());
    let pass = std::env::var("DB_PASSWORD").unwrap_or_else(|_| "".into());
    let name = std::env::var("DB_NAME").unwrap_or_else(|_| "slims9_bulians".into());

    format!("mysql://{}:{}@{}:{}/{}", user, pass, host, port, name)
}
