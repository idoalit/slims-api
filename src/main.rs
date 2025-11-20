mod auth;
mod config;
mod error;
mod jsonapi;
mod resources;

use std::net::SocketAddr;

use axum::{Json, Router, routing::{get, post}};
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    auth::extract_secret,
    auth::login,
    config::{AppConfig, AppState, init_pool},
    jsonapi::{JsonApiDocument, resource, single_document},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login,
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
        resources::biblios::create_biblio,
        resources::biblios::update_biblio,
        resources::biblios::delete_biblio,
        resources::contents::list_contents,
        resources::contents::get_content,
        resources::contents::get_content_by_path,
        resources::files::list_files,
        resources::files::get_file,
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
        resources::lookups::suppliers,
        resources::lookups::topics,
        resources::lookups::content_types,
        resources::lookups::media_types,
        resources::lookups::carrier_types,
        resources::lookups::relation_terms,
        resources::lookups::loan_rules,
        resources::visitors::list_visitors,
        resources::visitors::get_visitor,
        resources::settings::list_settings,
        resources::settings::get_setting,
    ),
    components(schemas(
        auth::LoginRequest,
        auth::AuthResponse,
        auth::Role,
        auth::ModuleAccess,
        auth::Permission,
        auth::ModulePermission,
        auth::Claims,
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
        resources::biblios::UpsertBiblio,
        resources::biblios::GmdInfo,
        resources::biblios::PublisherInfo,
        resources::biblios::LanguageInfo,
        resources::biblios::ContentTypeInfo,
        resources::biblios::MediaTypeInfo,
        resources::biblios::CarrierTypeInfo,
        resources::biblios::FrequencyInfo,
        resources::biblios::PlaceInfo,
        resources::biblios::ItemSummary,
        resources::biblios::AttachmentInfo,
        resources::biblios::BiblioRelationInfo,
        resources::biblios::AuthorInfo,
        resources::biblios::TopicInfo,
        resources::contents::Content,
        resources::files::FileObject,
        resources::files::FileBiblioAttachment,
        resources::files::FileResponse,
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
        resources::lookups::Supplier,
        resources::lookups::Topic,
        resources::lookups::ContentType,
        resources::lookups::MediaType,
        resources::lookups::CarrierType,
        resources::lookups::RelationTerm,
        resources::lookups::LoanRule,
        resources::visitors::Visitor,
        resources::settings::SettingResponse,
        jsonapi::JsonApiDocument,
        jsonapi::JsonApiError,
        jsonapi::JsonApiErrorDocument,
    )),
    tags(
        (name = "Auth", description = "Autentikasi"),
        (name = "Members", description = "Manajemen member"),
        (name = "Items", description = "Manajemen item"),
        (name = "Loans", description = "Sirkulasi"),
        (name = "Biblios", description = "Bibliografi"),
        (name = "Contents", description = "Konten halaman"),
        (name = "Files", description = "Manajemen berkas"),
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
    let jwt_secret = extract_secret(config.jwt_secret);
    let state = AppState { pool, jwt_secret };

    let app = build_router(state.clone());

    let addr: SocketAddr = config.bind_addr.parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::permissive();

    Router::new()
        .route("/health", get(health))
        .route("/auth/login", post(login))
        .nest("/members", resources::members::router())
        .nest("/items", resources::items::router())
        .nest("/loans", resources::loans::router())
        .nest("/biblios", resources::biblios::router())
        .nest("/lookups", resources::lookups::router())
        .nest("/visitors", resources::visitors::router())
        .nest("/files", resources::files::router())
        .nest("/contents", resources::contents::router())
        .nest("/settings", resources::settings::router())
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Health check", body = JsonApiDocument)),
    tag = "Health"
)]
async fn health() -> Json<JsonApiDocument> {
    Json(single_document(resource("health", "health", json!({ "status": "ok" }))))
}
