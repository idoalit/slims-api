mod analytics;
mod auth;
mod config;
mod error;
mod jsonapi;
mod mcp;
mod resources;
mod storage;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Json, Router,
    http::{HeaderValue, Method, header},
    middleware,
    routing::{get, post},
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    auth::{extract_secret, login, logout, mcp_auth_middleware, me, refresh},
    config::{AppConfig, AppState, init_pool},
    jsonapi::{JsonApiDocument, resource, single_document},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login,
        auth::refresh,
        auth::logout,
        auth::me,
        resources::dashboard::get_dashboard,
        health,
        resources::members::list_members,
        resources::members::get_member,
        resources::members::create_member,
        resources::members::update_member,
        resources::members::delete_member,
        resources::items::list_items,
        resources::items::get_item,
        resources::items::create_item,
        resources::items::update_item,
        resources::items::delete_item,
        resources::loans::list_loans,
        resources::loans::create_loan,
        resources::loans::return_loan,
        resources::biblios::list_biblios,
        resources::biblios::simple_search_biblios,
        resources::biblios::advanced_search_biblios,
        resources::biblios::get_biblio,
        resources::biblios::list_public_biblios,
        resources::biblios::search_public_biblios,
        resources::biblios::get_public_biblio,
        resources::biblios::create_biblio,
        resources::biblios::update_biblio,
        resources::biblios::delete_biblio,
        resources::contents::list_contents,
        resources::contents::get_content,
        resources::contents::get_content_by_path,
        resources::files::list_files,
        resources::files::get_file,
        resources::uploads::upload_bibliography_cover,
        resources::lookups::member_types,
        resources::lookups::coll_types,
        resources::lookups::locations,
        resources::lookups::languages,
        resources::lookups::gmds,
        resources::lookups::item_statuses,
        resources::lookups::frequencies,
        resources::lookups::modules,
        resources::lookups::places,
        resources::lookups::publishers,
        resources::lookups::authors,
        resources::lookups::authority_types,
        resources::lookups::authority_levels,
        resources::lookups::suppliers,
        resources::lookups::topics,
        resources::lookups::content_types,
        resources::lookups::media_types,
        resources::lookups::carrier_types,
        resources::lookups::relation_terms,
        resources::lookups::loan_rules,
        resources::lookups::get_member_type,
        resources::lookups::create_member_type,
        resources::lookups::update_member_type,
        resources::lookups::delete_member_type,
        resources::lookups::get_coll_type,
        resources::lookups::create_coll_type,
        resources::lookups::update_coll_type,
        resources::lookups::delete_coll_type,
        resources::lookups::get_location,
        resources::lookups::create_location,
        resources::lookups::update_location,
        resources::lookups::delete_location,
        resources::lookups::get_language,
        resources::lookups::create_language,
        resources::lookups::update_language,
        resources::lookups::delete_language,
        resources::lookups::get_gmd,
        resources::lookups::create_gmd,
        resources::lookups::update_gmd,
        resources::lookups::delete_gmd,
        resources::lookups::get_item_status,
        resources::lookups::create_item_status,
        resources::lookups::update_item_status,
        resources::lookups::delete_item_status,
        resources::lookups::get_frequency,
        resources::lookups::create_frequency,
        resources::lookups::update_frequency,
        resources::lookups::delete_frequency,
        resources::lookups::get_module,
        resources::lookups::create_module,
        resources::lookups::update_module,
        resources::lookups::delete_module,
        resources::lookups::get_place,
        resources::lookups::create_place,
        resources::lookups::update_place,
        resources::lookups::delete_place,
        resources::lookups::get_publisher,
        resources::lookups::create_publisher,
        resources::lookups::update_publisher,
        resources::lookups::delete_publisher,
        resources::lookups::get_author,
        resources::lookups::create_author,
        resources::lookups::update_author,
        resources::lookups::delete_author,
        resources::lookups::get_authority_type,
        resources::lookups::create_authority_type,
        resources::lookups::update_authority_type,
        resources::lookups::delete_authority_type,
        resources::lookups::get_authority_level,
        resources::lookups::create_authority_level,
        resources::lookups::update_authority_level,
        resources::lookups::delete_authority_level,
        resources::lookups::get_supplier,
        resources::lookups::create_supplier,
        resources::lookups::update_supplier,
        resources::lookups::delete_supplier,
        resources::lookups::get_topic,
        resources::lookups::create_topic,
        resources::lookups::update_topic,
        resources::lookups::delete_topic,
        resources::lookups::get_content_type,
        resources::lookups::create_content_type,
        resources::lookups::update_content_type,
        resources::lookups::delete_content_type,
        resources::lookups::get_media_type,
        resources::lookups::create_media_type,
        resources::lookups::update_media_type,
        resources::lookups::delete_media_type,
        resources::lookups::get_carrier_type,
        resources::lookups::create_carrier_type,
        resources::lookups::update_carrier_type,
        resources::lookups::delete_carrier_type,
        resources::lookups::get_relation_term,
        resources::lookups::create_relation_term,
        resources::lookups::update_relation_term,
        resources::lookups::delete_relation_term,
        resources::lookups::get_loan_rule,
        resources::lookups::create_loan_rule,
        resources::lookups::update_loan_rule,
        resources::lookups::delete_loan_rule,
        resources::visitors::list_visitors,
        resources::visitors::get_visitor,
        resources::settings::list_settings,
        resources::settings::get_setting,
    ),
    components(schemas(
        auth::LoginRequest,
        auth::AuthResponse,
        auth::CurrentUserResponse,
        auth::Role,
        auth::ModuleAccess,
        auth::Permission,
        auth::ModulePermission,
        auth::Claims,
        analytics::CirculationPoint,
        analytics::DdcPoint,
        analytics::TopBook,
        resources::dashboard::DashboardParams,
        resources::dashboard::DashboardPeriod,
        resources::dashboard::DashboardMetrics,
        resources::dashboard::DashboardResponse,
        resources::members::Member,
        resources::members::MemberTypeInfo,
        resources::members::MemberResponse,
        resources::members::CreateMember,
        resources::items::Item,
        resources::items::ItemResponse,
        resources::items::CreateItem,
        resources::items::BiblioSummary,
        resources::items::CollTypeSummary,
        resources::items::LocationSummary,
        resources::items::ItemStatusSummary,
        resources::items::LoanStatusSummary,
        resources::loans::Loan,
        resources::loans::LoanResponse,
        resources::loans::CreateLoan,
        resources::loans::LoanMember,
        resources::loans::LoanItem,
        resources::biblios::Biblio,
        resources::biblios::BiblioResponse,
        resources::biblios::PublicBiblio,
        resources::biblios::PublicBiblioResponse,
        resources::biblios::UpsertBiblio,
        resources::biblios::BiblioAuthorInput,
        resources::biblios::GmdInfo,
        resources::biblios::PublisherInfo,
        resources::biblios::LanguageInfo,
        resources::biblios::ContentTypeInfo,
        resources::biblios::MediaTypeInfo,
        resources::biblios::CarrierTypeInfo,
        resources::biblios::FrequencyInfo,
        resources::biblios::PlaceInfo,
        resources::biblios::ItemSummary,
        resources::biblios::PublicItemSummary,
        resources::biblios::AttachmentInfo,
        resources::biblios::BiblioRelationInfo,
        resources::biblios::AuthorInfo,
        resources::biblios::TopicInfo,
        resources::contents::Content,
        resources::files::FileObject,
        resources::files::FileBiblioAttachment,
        resources::files::FileResponse,
        resources::uploads::UploadResponse,
        resources::uploads::UploadCoverForm,
        resources::lookups::MemberType,
        resources::lookups::CollType,
        resources::lookups::Location,
        resources::lookups::Language,
        resources::lookups::Gmd,
        resources::lookups::ItemStatus,
        resources::lookups::Frequency,
        resources::lookups::Module,
        resources::lookups::Place,
        resources::lookups::Publisher,
        resources::lookups::Author,
        resources::lookups::AuthorityType,
        resources::lookups::AuthorityLevel,
        resources::lookups::Supplier,
        resources::lookups::Topic,
        resources::lookups::ContentType,
        resources::lookups::MediaType,
        resources::lookups::CarrierType,
        resources::lookups::RelationTerm,
        resources::lookups::LoanRule,
        resources::lookups::UpsertMemberType,
        resources::lookups::UpsertCollType,
        resources::lookups::UpsertLocation,
        resources::lookups::UpsertLanguage,
        resources::lookups::UpsertGmd,
        resources::lookups::UpsertItemStatus,
        resources::lookups::UpsertFrequency,
        resources::lookups::UpsertModule,
        resources::lookups::UpsertPlace,
        resources::lookups::UpsertPublisher,
        resources::lookups::UpsertAuthor,
        resources::lookups::UpsertAuthorityType,
        resources::lookups::UpsertAuthorityLevel,
        resources::lookups::UpsertSupplier,
        resources::lookups::UpsertTopic,
        resources::lookups::UpsertContentType,
        resources::lookups::UpsertMediaType,
        resources::lookups::UpsertCarrierType,
        resources::lookups::UpsertRelationTerm,
        resources::lookups::UpsertLoanRule,
        resources::visitors::Visitor,
        resources::settings::SettingResponse,
        jsonapi::JsonApiDocument,
        jsonapi::JsonApiError,
        jsonapi::JsonApiErrorDocument,
    )),
    tags(
        (name = "Auth", description = "Autentikasi"),
        (name = "Dashboard", description = "Ringkasan dan analitik admin"),
        (name = "Members", description = "Manajemen member"),
        (name = "Items", description = "Manajemen item"),
        (name = "Loans", description = "Sirkulasi"),
        (name = "Biblios", description = "Bibliografi"),
        (name = "Catalog", description = "Katalog bibliografi publik"),
        (name = "Contents", description = "Konten halaman"),
        (name = "Files", description = "Manajemen berkas"),
        (name = "Uploads", description = "Unggah dan simpan berkas"),
        (name = "Lookups", description = "Data referensi"),
        (name = "Visitors", description = "Kunjungan"),
        (name = "Settings", description = "Pengaturan"),
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some("JWT Bearer token"))
                    .build(),
            ),
        );
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env()?;
    let pool = init_pool(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;
    let file_storage = storage::FileStorage::from_env().await?;
    let jwt_secret = extract_secret(config.jwt_secret);
    let state = AppState {
        pool,
        jwt_secret,
        jwt_issuer: extract_secret(config.jwt_issuer),
        jwt_audience: extract_secret(config.jwt_audience),
        access_token_ttl: config.access_token_ttl,
        refresh_session_ttl: config.refresh_session_ttl,
        refresh_remember_ttl: config.refresh_remember_ttl,
        cookie_secure: config.cookie_secure,
        file_storage,
    };

    let app = build_router(state.clone());

    let addr: SocketAddr = config.bind_addr.parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

fn build_router(state: AppState) -> Router {
    let cors = cors_layer();
    let local_media_dir = state
        .file_storage
        .as_ref()
        .and_then(storage::FileStorage::local_root)
        .map(ToOwned::to_owned);

    let mcp_pool = state.pool.clone();
    let mcp_service = StreamableHttpService::new(
        move || Ok(mcp::LibraryMcpServer::new(mcp_pool.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    let mcp_protected = Router::new()
        .route_service("/mcp", mcp_service)
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            mcp_auth_middleware,
        ));

    let router = Router::new()
        .route("/health", get(health))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .nest("/dashboard", resources::dashboard::router())
        .nest("/members", resources::members::router())
        .nest("/items", resources::items::router())
        .nest("/loans", resources::loans::router())
        .nest("/biblios", resources::biblios::router())
        .nest("/catalog/biblios", resources::biblios::public_router())
        .nest("/lookups", resources::lookups::router())
        .nest("/visitors", resources::visitors::router())
        .nest("/files", resources::files::router())
        .nest("/uploads", resources::uploads::router())
        .nest("/contents", resources::contents::router())
        .nest("/settings", resources::settings::router())
        .merge(mcp_protected)
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let router = if let Some(local_media_dir) = local_media_dir {
        router.nest_service("/media", ServeDir::new(local_media_dir))
    } else {
        router
    };

    router
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

fn cors_layer() -> CorsLayer {
    let configured = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:5173,http://127.0.0.1:5173".into());
    let origins = configured
        .split(',')
        .filter_map(|origin| origin.trim().parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::HeaderName::from_static("x-requested-with"),
        ])
        .allow_credentials(true)
}

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Health check", body = JsonApiDocument)),
    tag = "Health"
)]
async fn health() -> Json<JsonApiDocument> {
    Json(single_document(resource(
        "health",
        "health",
        json!({ "status": "ok" }),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use sqlx::mysql::MySqlPoolOptions;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        AppState {
            pool: MySqlPoolOptions::new()
                .connect_lazy("mysql://root:password@127.0.0.1/test")
                .expect("lazy pool"),
            jwt_secret: extract_secret("test-secret".into()),
            jwt_issuer: extract_secret("slims-api".into()),
            jwt_audience: extract_secret("slims-admin".into()),
            access_token_ttl: std::time::Duration::from_secs(600),
            refresh_session_ttl: std::time::Duration::from_secs(43_200),
            refresh_remember_ttl: std::time::Duration::from_secs(2_592_000),
            cookie_secure: false,
            file_storage: None,
        }
    }

    #[test]
    fn openapi_marks_catalog_as_public_and_keeps_biblios_protected() {
        let document = serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI serializes");
        let paths = &document["paths"];

        for path in [
            "/catalog/biblios",
            "/catalog/biblios/search",
            "/catalog/biblios/{biblio_id}",
        ] {
            assert!(
                paths[path]["get"].is_object(),
                "missing OpenAPI path {path}"
            );
            assert!(
                paths[path]["get"].get("security").is_none(),
                "public catalog path {path} must not require bearerAuth"
            );
        }

        assert_eq!(
            paths["/biblios"]["get"]["security"][0]["bearerAuth"],
            serde_json::json!([])
        );

        for path in ["/auth/me", "/dashboard"] {
            assert_eq!(
                paths[path]["get"]["security"][0]["bearerAuth"],
                serde_json::json!([])
            );
        }
    }

    #[test]
    fn openapi_exposes_crud_for_all_lookup_resources() {
        let document = serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI serializes");
        let paths = &document["paths"];

        for resource in [
            "member-types",
            "coll-types",
            "locations",
            "languages",
            "gmd",
            "item-statuses",
            "frequencies",
            "modules",
            "places",
            "publishers",
            "suppliers",
            "topics",
            "content-types",
            "media-types",
            "carrier-types",
            "relation-terms",
            "loan-rules",
        ] {
            let collection = format!("/lookups/{resource}");
            let item = format!("/lookups/{resource}/{{id}}");

            assert!(
                paths[&collection]["get"].is_object(),
                "missing GET {collection}"
            );
            assert!(
                paths[&collection]["post"].is_object(),
                "missing POST {collection}"
            );
            assert!(paths[&item]["get"].is_object(), "missing GET {item}");
            assert!(paths[&item]["put"].is_object(), "missing PUT {item}");
            assert!(paths[&item]["delete"].is_object(), "missing DELETE {item}");
        }
    }

    #[tokio::test]
    async fn protected_admin_endpoints_reject_missing_token() {
        let app = build_router(test_state());

        for path in ["/auth/me", "/dashboard"] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
        }
    }

    #[tokio::test]
    async fn local_media_route_serves_uploaded_files_without_authentication() {
        let nonce = rand::random::<u64>();
        let root = std::env::temp_dir().join(format!("slims-media-route-test-{nonce}"));
        tokio::fs::create_dir_all(&root).await.unwrap();
        tokio::fs::write(root.join("cover.txt"), b"local cover")
            .await
            .unwrap();

        let mut state = test_state();
        state.file_storage = Some(storage::FileStorage::local_for_test(
            root.clone(),
            "http://localhost:3000/media".into(),
        ));
        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .uri("/media/cover.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"local cover");
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn cookie_auth_endpoints_require_csrf_header() {
        let app = build_router(test_state());

        for path in ["/auth/refresh", "/auth/logout"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
        }
    }
}
