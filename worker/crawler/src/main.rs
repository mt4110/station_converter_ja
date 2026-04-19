use anyhow::Result;
use clap::Parser;
use station_crawler::{run_n02_ingest_cycle, IngestReport, N02_INGEST_LOCK_NAME};
use station_shared::{
    config::AppConfig,
    db::{connect_any_pool, SqlDialect},
    job_lock::{acquire_job_lock, JobLockBusy},
};
use tokio::time::{sleep, Duration};
use tracing::{error, info};

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long, conflicts_with = "loop_mode")]
    once: bool,

    #[arg(long = "loop", conflicts_with = "once")]
    loop_mode: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let cli = Cli::parse();
    let config = AppConfig::from_env("station-crawler")?;
    let pool = connect_any_pool(&config.database_url).await?;
    let dialect = SqlDialect::from(&config.database_type);
    let run_loop = cli.loop_mode && !cli.once;

    if !run_loop {
        let report = run_once(&config, &pool, dialect).await?;
        info!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }

    info!(
        interval_seconds = config.update_interval_seconds,
        "starting crawler dev loop"
    );

    loop {
        match run_once(&config, &pool, dialect).await {
            Ok(report) => info!("{}", serde_json::to_string(&report)?),
            Err(err) if err.downcast_ref::<JobLockBusy>().is_some() => {
                info!(error = %err, "ingest lock busy, skipping dev-loop cycle");
            }
            Err(err) => error!(error = %err, "crawler cycle failed"),
        }

        sleep(Duration::from_secs(config.update_interval_seconds)).await;
    }
}

async fn run_once(
    config: &AppConfig,
    pool: &sqlx::AnyPool,
    dialect: SqlDialect,
) -> Result<IngestReport> {
    let _lock = acquire_job_lock(
        &config.job_lock_dir,
        N02_INGEST_LOCK_NAME,
        &config.service_name,
    )
    .await?;

    run_n02_ingest_cycle(config, pool, dialect).await
}
