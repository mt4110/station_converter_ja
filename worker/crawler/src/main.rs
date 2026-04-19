use anyhow::Result;
use clap::Parser;
use station_crawler::run_n02_ingest_cycle;
use station_shared::{
    config::AppConfig,
    db::{connect_any_pool, SqlDialect},
};
use tokio::time::{sleep, Duration};
use tracing::{error, info};

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
    let report = run_n02_ingest_cycle(config, pool, dialect).await?;
    info!("{}", serde_json::to_string(&report)?);
    Ok(())
}
