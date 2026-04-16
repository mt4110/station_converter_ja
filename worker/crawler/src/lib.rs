pub mod n02;

use anyhow::Result;
use sqlx::AnyPool;
use station_shared::{config::AppConfig, db::SqlDialect};
use tracing::info;

pub const DEFAULT_SOURCE_SNAPSHOT_URL: &str =
    "https://nlftp.mlit.go.jp/ksj/gml/data/N02/N02-24/N02-24_GML.zip";
pub const N02_INGEST_LOCK_NAME: &str = "ingest-n02";

pub async fn run_n02_ingest_cycle(
    config: &AppConfig,
    pool: &AnyPool,
    dialect: SqlDialect,
) -> Result<n02::IngestReport> {
    let source_url = config
        .source_snapshot_url
        .as_deref()
        .unwrap_or(DEFAULT_SOURCE_SNAPSHOT_URL);

    tokio::fs::create_dir_all(&config.temp_asset_dir).await?;

    let bytes = load_snapshot_bytes(source_url).await?;

    let filename = format!("snapshot-{}.zip", chrono::Utc::now().format("%Y%m%d%H%M%S"));
    let output_path = format!("{}/{}", config.temp_asset_dir, filename);

    tokio::fs::write(&output_path, &bytes).await?;

    n02::ingest_snapshot(pool, dialect, source_url, &output_path, &bytes).await
}

async fn load_snapshot_bytes(source_url: &str) -> Result<Vec<u8>> {
    if let Some(path) = source_url.strip_prefix("file://") {
        info!(source_path = %path, "loading source snapshot from local file");
        return Ok(tokio::fs::read(path).await?);
    }

    info!(source_url = %source_url, "downloading source snapshot");

    let response = reqwest::get(source_url).await?.error_for_status()?;
    Ok(response.bytes().await?.to_vec())
}
