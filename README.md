slims-rest-api
==============

Slims REST API in Rust using Axum + SQLx against the `slims9_bulian` MySQL schema. Provides auth with JWT and CRUD for members, biblios, items, loans, and lookups with pagination and optional eager includes via query params.

Requirements
- Rust stable toolchain (via rustup).
- MySQL running with the SLiMS schema (see `slims.sql`).

Configuration
- Copy `.env` and set:
  - `DB_HOST`, `DB_PORT`, `DB_USER`, `DB_PASSWORD`, `DB_NAME`
  - `JWT_SECRET` (required, at least 32 random bytes)
  - `JWT_ISSUER`, `JWT_AUDIENCE`
  - `ACCESS_TOKEN_TTL_SECONDS`, `REFRESH_SESSION_TTL_SECONDS`, `REFRESH_REMEMBER_TTL_SECONDS`
  - `COOKIE_SECURE` (`true` in production; `false` only for local HTTP)
  - `CORS_ALLOWED_ORIGINS` (comma-separated; set to the exact frontend origins)
  - `BIND_ADDR` (default `0.0.0.0:3000`)
  - `STORAGE_DRIVER` (`local`, `s3`, atau `disabled`), `STORAGE_PREFIX`, dan `STORAGE_PUBLIC_BASE_URL` untuk memilih penyimpanan sampul dan lampiran bibliografi
  - Driver `local` memakai `STORAGE_LOCAL_DIR` (default `storage/uploads`) dan `API_PUBLIC_URL`; berkas dilayani API melalui `/media`
  - Driver `s3` memakai `S3_BUCKET`, `S3_REGION`, `S3_ENDPOINT_URL`, `S3_PUBLIC_BASE_URL`, dan `S3_FORCE_PATH_STYLE`
  - Kredensial object storage mengikuti AWS SDK credential chain, termasuk `AWS_ACCESS_KEY_ID` dan `AWS_SECRET_ACCESS_KEY`
- The app builds a MySQL URL from those vars if `DATABASE_URL` is not provided.
- Pada deployment container, mount `STORAGE_LOCAL_DIR` sebagai persistent volume agar upload driver `local` tidak hilang ketika container diganti. Isi `API_PUBLIC_URL` dengan origin API yang dapat diakses browser.
- Untuk driver `s3`, URL publik harus mengarah ke bucket/CDN yang dapat dibaca publik agar sampul tampil di OPAC; API tidak mengubah ACL object.

Run
```bash
cargo run --bin slims-rest-api
```
Server listens on `BIND_ADDR`.

To run the MCP stdio server instead:
```bash
cargo run --bin mcp_stdio
```

API Overview (high level)
- `POST /auth/login` — returns a short-lived access JWT and sets a rotating HttpOnly refresh cookie.
- `POST /auth/refresh` — rotates the refresh session and returns a new access JWT; requires `X-Requested-With: XMLHttpRequest`.
- `POST /auth/logout` — revokes the refresh-token family and clears its cookie.
- `GET /auth/me` — validates a bearer token and returns the current user's role, module permissions, and expiry.
- `GET /dashboard?start_date=YYYY-MM-DD&end_date=YYYY-MM-DD&group_by=day|week|month` — permission-aware repository analytics.
- `GET /health`
- `GET /catalog/biblios` — public OPAC-safe bibliography list; no login required.
- `GET /catalog/biblios/search?q=...` — public search across title, authors, and topics.
- `GET /catalog/biblios/{biblio_id}` — public bibliography detail.
- `GET /members` — paginated, optional `include=member_type`.
- `GET /biblios` — paginated, optional `include=gmd,publisher,language,authors,topics`.
- `GET /items` — paginated, optional `include=biblio,coll_type,location,item_status`.
- `GET /loans` — paginated, optional `include=member,item`, plus create/return endpoints.
- `GET /contents` — paginated list of CMS contents and by ID or path.
- `GET /files` — paginated list of uploaded files, optional `include=biblios`.
- `POST /uploads/bibliography-covers` — multipart upload sampul (field `file`, JPEG/PNG/WebP/GIF, maksimal 5 MB) ke object storage.
- `POST /uploads/bibliography-attachments` — multipart upload satu lampiran (field `file`, `title`, dan `description`; maksimal 128 MB) ke storage dan tabel `files`. UI dapat memanggilnya berulang untuk multiple upload, lalu mengirim `file_id` pada relasi `attachments` bibliografi.
- `GET /visitors` — visitor log, paginated.
- `GET /settings` — list settings or fetch a key; supports nested paths via dot notation.
- `/lookups/*` — paginated lookup lists plus `GET /{id}`, `POST`, `PUT /{id}`, and `DELETE /{id}` for member types, collection types, locations, topics, content/media/carrier types, and other reference data.
- `GET /biblios/search` — simple search across title, authors, and topics with `q`, paginated and supports `include`.
- `POST /biblios/search/advanced` — advanced search with field-specific clauses and boolean logic.
- Standard CRUD for members, biblios, items, and lookup/reference resources; loans support create/return endpoints.
- OpenAPI docs + Swagger UI available at `/docs` (served from `/api-docs/openapi.json`).

Dashboard fields are returned as `null` when the authenticated user lacks read access to the corresponding module. Bibliography controls bibliography/item/DDC data, Membership controls member totals, and Circulation controls loan KPIs, trends, and popular books. Date ranges default to the latest 30 days and may span at most 366 days.

Public Catalog
--------------

The public catalog can be consumed without an `Authorization` header:

```bash
curl 'http://localhost:3000/catalog/biblios?page[number]=1&page[size]=10&sort=title&include=authors,publisher'
curl 'http://localhost:3000/catalog/biblios/search?q=rust&include=authors,topics'
curl 'http://localhost:3000/catalog/biblios/123?include=gmd,authors,items,attachments'
```

Only records with `opac_hide` unset or `0` are returned. Internal flags and timestamps, custom fields, private attachments, and relations to hidden records are never exposed. Supported public includes are `gmd`, `publisher`, `language`, `content_type`, `media_type`, `carrier_type`, `frequency`, `place`, `authors`, `topics`, `items`, `relations`, and `attachments` (alias: `files`). Unsupported includes return `400 Bad Request`; a missing or OPAC-hidden detail returns `404 Not Found`.

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
- Embedded SQLx migrations run at API startup. Production database credentials must be permitted to apply pending migrations, or migrations must be applied by the deployment pipeline before startup.

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
- CORS uses the `CORS_ALLOWED_ORIGINS` allowlist and defaults to local Vite development origins.
- Add data via SQL imports before running.
