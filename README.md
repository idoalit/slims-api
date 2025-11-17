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
- `GET /loans` — paginated, optional `include=member,item`.
- `GET /lookups/*` — paginated lookup lists (member-types, coll-types, locations, etc.).
- Standard CRUD for members, biblios, items; loans support create/return endpoints.

Pagination & Include
- Pagination query: `?page=1&per_page=20` (defaults: page=1, per_page=20, max 100).
- Includes: `?include=gmd,publisher` (comma-separated). Unknown includes are ignored; when omitted, base fields only are returned.

Database
- Schema dump: `slims.sql`.
- Uses MySQL via SQLx (runtime tokio + rustls).

Development notes
- Logging via `RUST_LOG`.
- CORS is permissive (adjust in `build_router` if needed).
- Add data via SQL imports before running.
