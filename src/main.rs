mod auth;
mod config;
mod error;
mod resources;

use std::net::SocketAddr;

use axum::{
    Router,
    response::Html,
    routing::{get, post},
};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

use crate::{
    auth::extract_secret,
    auth::login,
    config::{AppConfig, AppState, init_pool},
};

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
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn health() -> Html<&'static str> {
    Html("OK")
}
