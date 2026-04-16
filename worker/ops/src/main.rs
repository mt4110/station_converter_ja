mod export_sqlite;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use sqlx::{
    migrate::Migrator,
    mysql::MySqlPoolOptions,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use station_crawler::{run_n02_ingest_cycle, N02_INGEST_LOCK_NAME};
use station_shared::config::{AppConfig, DatabaseType};
use station_shared::{
    db::{connect_any_pool, SqlDialect},
    job_lock::try_acquire_job_lock,
};
use std::str::FromStr;
use tracing::info;

static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("../../storage/migrations/postgres");
static MYSQL_MIGRATOR: Migrator = sqlx::migrate!("../../storage/migrations/mysql");
static SQLITE_MIGRATOR: Migrator = sqlx::migrate!("../../storage/migrations/sqlite");

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Migrate,
    ExportSqlite,
    Job {
        #[command(subcommand)]
        job: Jobs,
    },
}

#[derive(Debug, Subcommand)]
enum Jobs {
    IngestN02 {
        #[arg(long)]
        export_sqlite: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let cli = Cli::parse();
    let config = AppConfig::from_env("station-ops")?;

    match cli.command {
        Commands::Migrate => migrate(&config).await?,
        Commands::ExportSqlite => {
            let _lock = try_acquire_job_lock(
                &config.job_lock_dir,
                N02_INGEST_LOCK_NAME,
                &config.service_name,
            )?;
            let report = export_sqlite::export_sqlite(&config).await?;
            info!(
                output_path = %report.output_path.display(),
                source_snapshots = report.source_snapshots,
                station_identities = report.station_identities,
                station_versions = report.station_versions,
                station_change_events = report.station_change_events,
                "sqlite artifact export complete"
            );
        }
        Commands::Job { job } => match job {
            Jobs::IngestN02 { export_sqlite } => run_ingest_n02_job(&config, export_sqlite).await?,
        },
    }

    Ok(())
}

async fn migrate(config: &AppConfig) -> Result<()> {
    match config.database_type {
        DatabaseType::Postgres => {
            let pool = PgPoolOptions::new().connect(&config.database_url).await?;
            POSTGRES_MIGRATOR.run(&pool).await?;
        }
        DatabaseType::Mysql => {
            let pool = MySqlPoolOptions::new()
                .connect(&config.database_url)
                .await?;
            MYSQL_MIGRATOR.run(&pool).await?;
        }
        DatabaseType::Sqlite => {
            let options = SqliteConnectOptions::from_str(&config.database_url)?
                .create_if_missing(true)
                .foreign_keys(true);
            let pool = SqlitePoolOptions::new().connect_with(options).await?;
            SQLITE_MIGRATOR.run(&pool).await?;
        }
    }

    info!("migrations complete for {}", config.database_type);
    Ok(())
}

async fn run_ingest_n02_job(config: &AppConfig, chain_export_sqlite: bool) -> Result<()> {
    if chain_export_sqlite && matches!(config.database_type, DatabaseType::Sqlite) {
        bail!("--export-sqlite expects DATABASE_TYPE to be postgres or mysql");
    }

    let pool = connect_any_pool(&config.database_url).await?;
    let dialect = SqlDialect::from(&config.database_type);
    let _ingest_lock = try_acquire_job_lock(
        &config.job_lock_dir,
        N02_INGEST_LOCK_NAME,
        &config.service_name,
    )?;

    let report = run_n02_ingest_cycle(config, &pool, dialect).await?;
    info!(
        source_name = report.source_name,
        source_version = report.source_version.as_deref().unwrap_or("unknown"),
        source_url = %report.source_url,
        source_sha256 = %report.source_sha256,
        saved_to = %report.saved_to,
        snapshot_id = report.snapshot_id.unwrap_or_default(),
        parsed_features = report.parsed_features,
        parsed_stations = report.parsed_stations,
        created = report.created,
        updated = report.updated,
        unchanged = report.unchanged,
        removed = report.removed,
        skipped_existing_snapshot = report.skipped_existing_snapshot,
        "ingest-n02 job complete"
    );

    if chain_export_sqlite {
        let export_report = export_sqlite::export_sqlite(config).await?;
        info!(
            output_path = %export_report.output_path.display(),
            source_snapshots = export_report.source_snapshots,
            station_identities = export_report.station_identities,
            station_versions = export_report.station_versions,
            station_change_events = export_report.station_change_events,
            "ingest-n02 chained sqlite export complete"
        );
    }

    Ok(())
}
