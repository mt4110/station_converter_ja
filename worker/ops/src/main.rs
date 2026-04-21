mod export_sqlite;
mod validate_ingest;

use std::{process::ExitCode, str::FromStr};

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use sqlx::{
    migrate::Migrator,
    mysql::MySqlPoolOptions,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use station_crawler::{run_n02_ingest_cycle, N02_INGEST_LOCK_NAME};
use station_shared::{
    config::{AppConfig, DatabaseType},
    db::{connect_any_pool, SqlDialect},
    job_lock::acquire_job_lock,
};
use tracing::info;
use validate_ingest::{render_validation_report, validate_ingest, ValidateIngestArgs};

static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("../../storage/migrations/postgres");
static MYSQL_MIGRATOR: Migrator = sqlx::migrate!("../../storage/migrations/mysql");
static SQLITE_MIGRATOR: Migrator = sqlx::migrate!("../../storage/migrations/sqlite");
const VERIFY_RESET_TABLES: [&str; 4] = [
    "station_change_events",
    "station_versions",
    "station_identities",
    "source_snapshots",
];

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Migrate,
    ResetVerifyDb {
        #[arg(long)]
        yes: bool,
    },
    ExportSqlite,
    ValidateIngest(ValidateIngestArgs),
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
async fn main() -> ExitCode {
    tracing_subscriber::fmt().with_env_filter("info").init();

    match run().await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let config = AppConfig::from_env("station-ops")?;

    match cli.command {
        Commands::Migrate => migrate(&config).await?,
        Commands::ResetVerifyDb { yes } => reset_verify_db(&config, yes).await?,
        Commands::ExportSqlite => {
            // Export shares the ingest lock so SQLite snapshots never race a live ingest.
            let _lock = acquire_job_lock(
                &config.job_lock_dir,
                N02_INGEST_LOCK_NAME,
                &config.service_name,
            )
            .await?;
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
        Commands::ValidateIngest(args) => return run_validate_ingest(&config, args).await,
        Commands::Job { job } => match job {
            Jobs::IngestN02 { export_sqlite } => run_ingest_n02_job(&config, export_sqlite).await?,
        },
    }

    Ok(ExitCode::SUCCESS)
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

async fn reset_verify_db(config: &AppConfig, yes: bool) -> Result<()> {
    if !yes {
        bail!("reset-verify-db is destructive; re-run with --yes");
    }

    match config.database_type {
        DatabaseType::Postgres => {
            let pool = PgPoolOptions::new().connect(&config.database_url).await?;
            sqlx::query(
                "TRUNCATE TABLE
                   station_change_events,
                   station_versions,
                   station_identities,
                   source_snapshots
                 RESTART IDENTITY CASCADE",
            )
            .execute(&pool)
            .await?;
        }
        DatabaseType::Mysql => {
            let pool = MySqlPoolOptions::new()
                .connect(&config.database_url)
                .await?;
            let mut conn = pool.acquire().await?;

            // Use TRUNCATE so verification reruns start from deterministic AUTO_INCREMENT values.
            sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
                .execute(&mut *conn)
                .await?;

            let reset_result = async {
                for table in VERIFY_RESET_TABLES {
                    let statement = format!("TRUNCATE TABLE {table}");
                    sqlx::query(&statement).execute(&mut *conn).await?;
                }
                Ok::<(), sqlx::Error>(())
            }
            .await;

            let restore_result = sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
                .execute(&mut *conn)
                .await;

            if let Err(error) = reset_result {
                restore_result?;
                return Err(error.into());
            }

            restore_result?;
        }
        DatabaseType::Sqlite => {
            bail!("reset-verify-db is only supported for postgres or mysql");
        }
    }

    info!("verification database reset for {}", config.database_type);
    Ok(())
}

async fn run_ingest_n02_job(config: &AppConfig, chain_export_sqlite: bool) -> Result<()> {
    if chain_export_sqlite && matches!(config.database_type, DatabaseType::Sqlite) {
        bail!("--export-sqlite expects DATABASE_TYPE to be postgres or mysql");
    }

    let _ingest_lock = acquire_job_lock(
        &config.job_lock_dir,
        N02_INGEST_LOCK_NAME,
        &config.service_name,
    )
    .await?;
    let pool = connect_any_pool(&config.database_url).await?;
    let dialect = SqlDialect::from(&config.database_type);

    let report = run_n02_ingest_cycle(config, &pool, dialect).await?;
    let snapshot_id = report
        .snapshot_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "none".to_string());
    info!(
        source_name = report.source_name,
        source_version = report.source_version.as_deref().unwrap_or("unknown"),
        source_url = %report.source_url,
        source_sha256 = %report.source_sha256,
        saved_to = %report.saved_to,
        snapshot_id,
        parsed_features = report.parsed_features,
        parsed_stations = report.parsed_stations,
        created = report.created,
        updated = report.updated,
        unchanged = report.unchanged,
        removed = report.removed,
        skipped_existing_snapshot = report.skipped_existing_snapshot,
        load_ms = report.load_ms,
        save_zip_ms = report.save_zip_ms,
        extract_ms = report.extract_ms,
        parse_ms = report.parse_ms,
        diff_ms = report.diff_ms,
        persist_ms = report.persist_ms,
        total_ms = report.total_ms,
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

async fn run_validate_ingest(config: &AppConfig, args: ValidateIngestArgs) -> Result<ExitCode> {
    let _ingest_lock = acquire_job_lock(
        &config.job_lock_dir,
        N02_INGEST_LOCK_NAME,
        &config.service_name,
    )
    .await?;
    let pool = connect_any_pool(&config.database_url).await?;
    let dialect = SqlDialect::from(&config.database_type);
    let report = validate_ingest(&pool, dialect, &args).await?;
    let output = render_validation_report(&report, args.json)?;

    println!("{output}");

    Ok(report.exit_code())
}
