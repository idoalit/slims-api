slims-rest-api
==============

Slims REST API in Rust using Axum + SQLx against the `slims9_bulian` MySQL schema. Provides auth with JWT and CRUD for members, biblios, items, loans, and lookups with pagination and optional eager includes via query params.

Requirements
- Rust stable toolchain (via rustup).
- MySQL running with the SLiMS schema (see `slims.sql`).

Configuration
- Copy `.env` and set:
  - `DB_HOST`, `DB_PORT`, `DB_USER`, `DB_PASSWORD`, `DB_NAME`
  - `JWT_SECRET`
  - `BIND_ADDR` (default `0.0.0.0:3000`)
- The app builds a MySQL URL from those vars if `DATABASE_URL` is not provided.

Run
```bash
cargo run
```
Server listens on `BIND_ADDR`.

API Overview (high level)
- `POST /auth/login` — returns JWT.
- `GET /health`
- `GET /members` — paginated, optional `include=member_type`.
- `GET /biblios` — paginated, optional `include=gmd,publisher,language,authors,topics`.
- `GET /items` — paginated, optional `include=biblio,coll_type,location,item_status`.
- `GET /loans` — paginated, optional `include=member,item`, plus create/return endpoints.
- `GET /contents` — paginated list of CMS contents and by ID or path.
- `GET /files` — paginated list of uploaded files, optional `include=biblios`.
- `GET /visitors` — visitor log, paginated.
- `GET /settings` — list settings or fetch a key; supports nested paths via dot notation.
- `GET /lookups/*` — paginated lookup lists (member-types, coll-types, locations, topics, content/media/carrier types, etc.).
- `GET /biblios/search` — simple search across title, authors, and topics with `q`, paginated and supports `include`.
- `POST /biblios/search/advanced` — advanced search with field-specific clauses and boolean logic.
- Standard CRUD for members, biblios, items; loans support create/return endpoints.
- OpenAPI docs + Swagger UI available at `/docs` (served from `/api-docs/openapi.json`).

Pagination & Include
- Pagination query: `?page=1&per_page=20` (defaults: page=1, per_page=20, max 100).
- Includes: `?include=gmd,publisher` (comma-separated). Unknown includes are ignored; when omitted, base fields only are returned.

Search
- Simple search: `GET /biblios/search?q=rust&page=1&per_page=10&include=authors,topics` (keyword matches title, author, topic).
- Advanced search: `POST /biblios/search/advanced` with JSON body:

```json
{
  "clauses": [
    { "field": "title", "value": "rust", "type": "contains" },
    { "field": "author", "value": "gray", "op": "or" },
    { "field": "publisher", "value": "oreilly", "op": "and", "type": "starts_with" }
  ],
  "page": 1,
  "per_page": 20,
  "include": "authors,topics"
}
```

Database
- Schema dump: `slims.sql`.
- Uses MySQL via SQLx (runtime tokio + rustls).

MCP (Model Context Protocol) Support
--------------------------------------

The API supports [MCP](https://modelcontextprotocol.io) so AI agents (Claude, Cursor, etc.) can interact with the library system directly.

### Transports

**Streamable HTTP** — built into the main server, protected by JWT.

```
POST/GET/DELETE http://localhost:3000/mcp
Authorization: Bearer <jwt-token>
```

Obtain a token first via `POST /auth/login`, then pass it as a `Bearer` token.

**stdio** — separate binary for local AI agent integrations (Claude Desktop, Claude Code, Cursor).  
Auth is not required; it connects to the DB directly from the local process.

### Stdio binary: Claude Desktop / Cursor config

Build the release binary first:
```bash
cargo build --release
```

Add to your Claude Desktop `claude_desktop_config.json` (usually at `~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "slims-library": {
      "command": "/absolute/path/to/target/release/mcp_stdio",
      "env": {
        "DB_HOST": "localhost",
        "DB_PORT": "3306",
        "DB_USER": "root",
        "DB_PASSWORD": "yourpassword",
        "DB_NAME": "slims9_bulians"
      }
    }
  }
}
```

### Available MCP Tools

| Tool | Description |
|------|-------------|
| `library_catalog_search` | Search books by title, author, subject, or ISBN |
| `library_catalog_get_biblio_detail` | Get full bibliographic details, item copies, and availability |
| `library_items_list` | List physical item copies with optional location and availability filters |
| `library_members_search` | Search members by name, member ID, or email |
| `library_members_get_detail` | Get member details including membership type and active loan count |
| `library_loans_list` | List loans with optional filters by member, item code, or active status |
| `library_loans_checkout_create` | Create a checkout transaction (validates availability and member status) |
| `library_loans_return_register` | Register an item return |
| `library_lookups_list` | Retrieve lookup/reference data (locations, GMDs, languages, collection types, etc.) |
| `library_reports_circulation` | Circulation report (loaned, returned, active, overdue) plus most borrowed titles |
| `library_reports_overdue_loans` | Overdue loans report with estimated fines |
| `library_reports_collection_overview` | Collection overview with totals and breakdown by GMD/location/collection type |
| `library_reports_member_overview` | Member statistics by type plus top borrowers |
| `library_reports_visitor_overview` | Visitor report by day or month |
| `library_reports_fines_overview` | Member fines report (debit/credit/outstanding) |
| `library_reports_collection_growth` | Collection growth report within a selected date range |

Development notes
- Logging via `RUST_LOG`.
- CORS is permissive (adjust in `build_router` if needed).
- Add data via SQL imports before running.
