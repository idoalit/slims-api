use std::sync::Arc;

use anyhow::Context;
use dotenvy::dotenv;
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};

#[derive(Clone)]
pub struct AppState {
    pub pool: MySqlPool,
    pub jwt_secret: Arc<str>,
}

#[derive(Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub bind_addr: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenv().ok();

        let db_host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".into());
        let db_port = std::env::var("DB_PORT").unwrap_or_else(|_| "3306".into());
        let db_user = std::env::var("DB_USER").unwrap_or_else(|_| "root".into());
        let db_pass = std::env::var("DB_PASSWORD").unwrap_or_else(|_| "ServBay.dev".into());
        let db_name = std::env::var("DB_NAME").unwrap_or_else(|_| "slims9_bulians".into());

        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            format!(
                "mysql://{user}:{pass}@{host}:{port}/{db}",
                user = db_user,
                pass = db_pass,
                host = db_host,
                port = db_port,
                db = db_name
            )
        });

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-please".into());
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());

        Ok(Self {
            database_url,
            jwt_secret,
            bind_addr,
        })
    }
}

pub async fn init_pool(database_url: &str) -> anyhow::Result<MySqlPool> {
    MySqlPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(database_url)
        .await
        .with_context(|| "failed to connect to MySQL")
}
