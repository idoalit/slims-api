use anyhow::{Context, bail};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client, config::Region, primitives::ByteStream};

#[derive(Clone)]
pub struct ObjectStorage {
    client: Client,
    bucket: String,
    public_base_url: String,
    prefix: String,
}

impl ObjectStorage {
    pub async fn from_env() -> anyhow::Result<Option<Self>> {
        let bucket = match optional_env("S3_BUCKET") {
            Some(value) => value,
            None => return Ok(None),
        };
        let region = optional_env("S3_REGION").unwrap_or_else(|| "us-east-1".into());
        let endpoint = optional_env("S3_ENDPOINT_URL");
        let force_path_style = bool_env("S3_FORCE_PATH_STYLE", endpoint.is_some())?;
        let prefix = optional_env("S3_PREFIX")
            .unwrap_or_else(|| "bibliography/covers".into())
            .trim_matches('/')
            .to_string();
        validate_prefix(&prefix)?;

        let public_base_url = optional_env("S3_PUBLIC_BASE_URL")
            .or_else(|| {
                endpoint
                    .as_ref()
                    .map(|value| format!("{}/{bucket}", value.trim_end_matches('/')))
            })
            .unwrap_or_else(|| format!("https://{bucket}.s3.{region}.amazonaws.com"))
            .trim_end_matches('/')
            .to_string();
        if !public_base_url.starts_with("https://") && !public_base_url.starts_with("http://") {
            bail!("S3_PUBLIC_BASE_URL harus berupa URL http atau https");
        }
        let prefix_separator_length = if prefix.is_empty() { 0 } else { 1 };
        let generated_url_length =
            public_base_url.len() + 1 + prefix.len() + prefix_separator_length + 36;
        if generated_url_length > 512 {
            bail!("kombinasi S3_PUBLIC_BASE_URL dan S3_PREFIX terlalu panjang");
        }

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region))
            .load()
            .await;
        let mut s3_config =
            aws_sdk_s3::config::Builder::from(&shared_config).force_path_style(force_path_style);
        if let Some(endpoint) = endpoint {
            s3_config = s3_config.endpoint_url(endpoint);
        }

        Ok(Some(Self {
            client: Client::from_conf(s3_config.build()),
            bucket,
            public_base_url,
            prefix,
        }))
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
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(bytes))
            .content_type(content_type)
            .cache_control("public, max-age=31536000, immutable")
            .send()
            .await
            .with_context(|| format!("gagal mengunggah object {key}"))?;

        let url = format!("{}/{key}", self.public_base_url);
        Ok((key, url))
    }
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

fn validate_prefix(prefix: &str) -> anyhow::Result<()> {
    if prefix.split('/').any(|part| {
        part == "."
            || part == ".."
            || !part
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    }) {
        bail!(
            "S3_PREFIX hanya boleh berisi segmen alfanumerik, titik, garis bawah, atau tanda hubung"
        );
    }
    Ok(())
}
