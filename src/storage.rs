use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, bail};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client, config::Region, primitives::ByteStream};
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
pub struct FileStorage {
    driver: StorageDriver,
    public_base_url: String,
    prefix: String,
}

#[derive(Clone)]
enum StorageDriver {
    Local { root: Arc<PathBuf> },
    S3 { client: Client, bucket: String },
}

impl FileStorage {
    pub async fn from_env() -> anyhow::Result<Option<Self>> {
        let driver_name = optional_env("STORAGE_DRIVER").unwrap_or_else(|| {
            if optional_env("S3_BUCKET").is_some() {
                "s3".into()
            } else {
                "local".into()
            }
        });
        if driver_name == "disabled" {
            return Ok(None);
        }

        let prefix = optional_env("STORAGE_PREFIX")
            .or_else(|| optional_env("S3_PREFIX"))
            .unwrap_or_else(|| "bibliography/covers".into())
            .trim_matches('/')
            .to_string();
        validate_prefix(&prefix)?;

        let (driver, default_public_base_url) = match driver_name.as_str() {
            "local" => local_driver().await?,
            "s3" => s3_driver().await?,
            _ => bail!("STORAGE_DRIVER harus bernilai local, s3, atau disabled"),
        };
        let public_base_url = optional_env("STORAGE_PUBLIC_BASE_URL")
            .unwrap_or(default_public_base_url)
            .trim_end_matches('/')
            .to_string();
        validate_public_url(&public_base_url, &prefix)?;

        Ok(Some(Self {
            driver,
            public_base_url,
            prefix,
        }))
    }

    pub fn local_root(&self) -> Option<&Path> {
        match &self.driver {
            StorageDriver::Local { root } => Some(root.as_path()),
            StorageDriver::S3 { .. } => None,
        }
    }

    #[cfg(test)]
    pub fn local_for_test(root: PathBuf, public_base_url: String) -> Self {
        Self {
            driver: StorageDriver::Local {
                root: Arc::new(root),
            },
            public_base_url,
            prefix: String::new(),
        }
    }

    pub async fn put(
        &self,
        key_suffix: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> anyhow::Result<(String, String)> {
        let key = if self.prefix.is_empty() {
            key_suffix.to_string()
        } else {
            format!("{}/{key_suffix}", self.prefix)
        };

        match &self.driver {
            StorageDriver::Local { root } => write_local(root, &key, &bytes).await?,
            StorageDriver::S3 { client, bucket } => {
                client
                    .put_object()
                    .bucket(bucket)
                    .key(&key)
                    .body(ByteStream::from(bytes))
                    .content_type(content_type)
                    .cache_control("public, max-age=31536000, immutable")
                    .send()
                    .await
                    .with_context(|| format!("gagal mengunggah object {key}"))?;
            }
        }

        let url = format!("{}/{key}", self.public_base_url);
        Ok((key, url))
    }
}

async fn local_driver() -> anyhow::Result<(StorageDriver, String)> {
    let root = PathBuf::from(
        optional_env("STORAGE_LOCAL_DIR").unwrap_or_else(|| "storage/uploads".into()),
    );
    tokio::fs::create_dir_all(&root)
        .await
        .with_context(|| format!("gagal membuat direktori upload lokal {}", root.display()))?;
    let api_public_url =
        optional_env("API_PUBLIC_URL").unwrap_or_else(|| "http://localhost:3000".into());
    Ok((
        StorageDriver::Local {
            root: Arc::new(root),
        },
        format!("{}/media", api_public_url.trim_end_matches('/')),
    ))
}

async fn s3_driver() -> anyhow::Result<(StorageDriver, String)> {
    let bucket =
        optional_env("S3_BUCKET").context("S3_BUCKET wajib diatur ketika STORAGE_DRIVER=s3")?;
    let region = optional_env("S3_REGION").unwrap_or_else(|| "us-east-1".into());
    let endpoint = optional_env("S3_ENDPOINT_URL");
    let force_path_style = bool_env("S3_FORCE_PATH_STYLE", endpoint.is_some())?;
    let shared_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region.clone()))
        .load()
        .await;
    let mut s3_config =
        aws_sdk_s3::config::Builder::from(&shared_config).force_path_style(force_path_style);
    if let Some(endpoint) = &endpoint {
        s3_config = s3_config.endpoint_url(endpoint);
    }
    let public_base_url = optional_env("S3_PUBLIC_BASE_URL")
        .or_else(|| {
            endpoint
                .as_ref()
                .map(|value| format!("{}/{bucket}", value.trim_end_matches('/')))
        })
        .unwrap_or_else(|| format!("https://{bucket}.s3.{region}.amazonaws.com"));

    Ok((
        StorageDriver::S3 {
            client: Client::from_conf(s3_config.build()),
            bucket,
        },
        public_base_url,
    ))
}

async fn write_local(root: &Path, key: &str, bytes: &[u8]) -> anyhow::Result<()> {
    let path = root.join(key);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("gagal membuat direktori upload {}", parent.display()))?;
    }
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await
        .with_context(|| format!("gagal membuat berkas upload {}", path.display()))?;
    file.write_all(bytes)
        .await
        .with_context(|| format!("gagal menulis berkas upload {}", path.display()))?;
    file.flush()
        .await
        .with_context(|| format!("gagal menyelesaikan upload {}", path.display()))?;
    Ok(())
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn bool_env(name: &str, default: bool) -> anyhow::Result<bool> {
    optional_env(name)
        .map(|value| {
            value
                .parse::<bool>()
                .with_context(|| format!("{name} harus bernilai true atau false"))
        })
        .unwrap_or(Ok(default))
}

fn validate_public_url(public_base_url: &str, prefix: &str) -> anyhow::Result<()> {
    if !public_base_url.starts_with("https://") && !public_base_url.starts_with("http://") {
        bail!("STORAGE_PUBLIC_BASE_URL harus berupa URL http atau https");
    }
    let prefix_separator_length = if prefix.is_empty() { 0 } else { 1 };
    let generated_url_length =
        public_base_url.len() + 1 + prefix.len() + prefix_separator_length + 36;
    if generated_url_length > 512 {
        bail!("kombinasi STORAGE_PUBLIC_BASE_URL dan STORAGE_PREFIX terlalu panjang");
    }
    Ok(())
}

fn validate_prefix(prefix: &str) -> anyhow::Result<()> {
    if prefix.split('/').any(|part| {
        part == "."
            || part == ".."
            || !part
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    }) {
        bail!(
            "STORAGE_PREFIX hanya boleh berisi segmen alfanumerik, titik, garis bawah, atau tanda hubung"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_driver_writes_generated_key_below_root() {
        let nonce = rand::random::<u64>();
        let root = std::env::temp_dir().join(format!("slims-storage-test-{nonce}"));
        let key = format!("covers/{nonce}.txt");
        write_local(&root, &key, b"cover").await.unwrap();
        assert_eq!(tokio::fs::read(root.join(&key)).await.unwrap(), b"cover");
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[test]
    fn rejects_parent_segments_in_storage_prefix() {
        assert!(validate_prefix("bibliography/covers").is_ok());
        assert!(validate_prefix("../covers").is_err());
    }
}
