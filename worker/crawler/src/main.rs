mod n02;

use anyhow::Result;
use clap::Parser;
use station_shared::{
    config::AppConfig,
    db::{connect_any_pool, SqlDialect},
};
use tokio::time::{sleep, Duration};
use tracing::{error, info};

const DEFAULT_SOURCE_SNAPSHOT_URL: &str =
    "https://nlftp.mlit.go.jp/ksj/gml/data/N02/N02-24/N02-24_GML.zip";

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long)]
    once: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let cli = Cli::parse();
    let config = AppConfig::from_env("station-crawler")?;
    let pool = connect_any_pool(&config.database_url).await?;
    let dialect = SqlDialect::from(&config.database_type);

    loop {
        match run_once(&config, &pool, dialect).await {
            Ok(()) => info!("crawler cycle complete"),
            Err(err) => error!(error = %err, "crawler cycle failed"),
        }

        if cli.once {
            break;
        }

        sleep(Duration::from_secs(config.update_interval_seconds)).await;
    }

    Ok(())
}

async fn run_once(config: &AppConfig, pool: &sqlx::AnyPool, dialect: SqlDialect) -> Result<()> {
    let source_url = config
        .source_snapshot_url
        .as_deref()
        .unwrap_or(DEFAULT_SOURCE_SNAPSHOT_URL);

    tokio::fs::create_dir_all(&config.temp_asset_dir).await?;

    info!(source_url = %source_url, "downloading source snapshot");

    let response = reqwest::get(source_url).await?.error_for_status()?;
    let bytes = response.bytes().await?;

    let filename = format!("snapshot-{}.zip", chrono::Utc::now().format("%Y%m%d%H%M%S"));
    let output_path = format!("{}/{}", config.temp_asset_dir, filename);

    tokio::fs::write(&output_path, &bytes).await?;

    let report =
        n02::ingest_snapshot(pool, dialect, source_url, &output_path, bytes.as_ref()).await?;

    info!("{}", serde_json::to_string(&report)?);
    Ok(())
}
