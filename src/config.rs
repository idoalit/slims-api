use std::{sync::Arc, time::Duration};

use anyhow::{Context, bail};
use dotenvy::dotenv;
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};

use crate::storage::FileStorage;

#[derive(Clone)]
pub struct AppState {
    pub pool: MySqlPool,
    pub jwt_secret: Arc<str>,
    pub jwt_issuer: Arc<str>,
    pub jwt_audience: Arc<str>,
    pub access_token_ttl: Duration,
    pub refresh_session_ttl: Duration,
    pub refresh_remember_ttl: Duration,
    pub cookie_secure: bool,
    pub file_storage: Option<FileStorage>,
}

#[derive(Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub access_token_ttl: Duration,
    pub refresh_session_ttl: Duration,
    pub refresh_remember_ttl: Duration,
    pub cookie_secure: bool,
    pub bind_addr: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenv().ok();

        let database_url = match std::env::var("DATABASE_URL") {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => {
                let db_host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".into());
                let db_port = std::env::var("DB_PORT").unwrap_or_else(|_| "3306".into());
                let db_user = std::env::var("DB_USER")
                    .context("DB_USER wajib diatur ketika DATABASE_URL tidak digunakan")?;
                let db_pass = std::env::var("DB_PASSWORD")
                    .context("DB_PASSWORD wajib diatur ketika DATABASE_URL tidak digunakan")?;
                let db_name = std::env::var("DB_NAME")
                    .context("DB_NAME wajib diatur ketika DATABASE_URL tidak digunakan")?;
                format!("mysql://{db_user}:{db_pass}@{db_host}:{db_port}/{db_name}")
            }
            Err(error) => return Err(error).context("gagal membaca DATABASE_URL"),
        };

        let jwt_secret = std::env::var("JWT_SECRET")
            .context("JWT_SECRET wajib diatur dan harus berupa secret acak minimal 32 byte")?;
        if jwt_secret.len() < 32 {
            bail!("JWT_SECRET harus memiliki panjang minimal 32 byte");
        }

        let jwt_issuer = std::env::var("JWT_ISSUER").unwrap_or_else(|_| "slims-api".into());
        let jwt_audience = std::env::var("JWT_AUDIENCE").unwrap_or_else(|_| "slims-admin".into());
        let access_token_ttl = duration_from_env("ACCESS_TOKEN_TTL_SECONDS", 10 * 60)?;
        let refresh_session_ttl = duration_from_env("REFRESH_SESSION_TTL_SECONDS", 12 * 60 * 60)?;
        let refresh_remember_ttl =
            duration_from_env("REFRESH_REMEMBER_TTL_SECONDS", 30 * 24 * 60 * 60)?;

        if !(60..=60 * 60).contains(&access_token_ttl.as_secs()) {
            bail!("ACCESS_TOKEN_TTL_SECONDS harus berada antara 60 dan 3600 detik");
        }
        if refresh_session_ttl < access_token_ttl
            || refresh_session_ttl > Duration::from_secs(24 * 60 * 60)
        {
            bail!("REFRESH_SESSION_TTL_SECONDS harus antara TTL access token dan 86400 detik");
        }
        if refresh_remember_ttl < refresh_session_ttl
            || refresh_remember_ttl > Duration::from_secs(90 * 24 * 60 * 60)
        {
            bail!("REFRESH_REMEMBER_TTL_SECONDS harus antara TTL sesi dan 7776000 detik");
        }

        let cookie_secure = bool_from_env("COOKIE_SECURE", true)?;
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());

        Ok(Self {
            database_url,
            jwt_secret,
            jwt_issuer,
            jwt_audience,
            access_token_ttl,
            refresh_session_ttl,
            refresh_remember_ttl,
            cookie_secure,
            bind_addr,
        })
    }
}

fn duration_from_env(name: &str, default_seconds: u64) -> anyhow::Result<Duration> {
    let seconds = match std::env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .with_context(|| format!("{name} harus berupa jumlah detik"))?,
        Err(std::env::VarError::NotPresent) => default_seconds,
        Err(error) => return Err(error).with_context(|| format!("gagal membaca {name}")),
    };
    Ok(Duration::from_secs(seconds))
}

fn bool_from_env(name: &str, default: bool) -> anyhow::Result<bool> {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<bool>()
            .with_context(|| format!("{name} harus bernilai true atau false")),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(error).with_context(|| format!("gagal membaca {name}")),
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
