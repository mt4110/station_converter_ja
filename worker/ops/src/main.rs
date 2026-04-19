mod validate_ingest;

use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use sqlx::{
    migrate::Migrator, mysql::MySqlPoolOptions, postgres::PgPoolOptions, sqlite::SqlitePoolOptions,
};
use station_crawler::run_n02_ingest_cycle;
use station_shared::{
    config::{AppConfig, DatabaseType},
    db::{connect_any_pool, SqlDialect},
};
use tracing::info;
use validate_ingest::{render_validation_report, validate_ingest, ValidateIngestArgs};

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
        Commands::ExportSqlite => {
            info!(
                "export-sqlite is a reserved command. Real export is not implemented in this scaffold."
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
            let pool = SqlitePoolOptions::new()
                .connect(&config.database_url)
                .await?;
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
    let report = run_n02_ingest_cycle(config, &pool, dialect).await?;

    info!(?report, "ingest-n02 job complete");

    if chain_export_sqlite {
        info!(
            "ingest completed; --export-sqlite was requested, but SQLite export remains reserved in this scaffold."
        );
    }

    Ok(())
}

async fn run_validate_ingest(config: &AppConfig, args: ValidateIngestArgs) -> Result<ExitCode> {
    let pool = connect_any_pool(&config.database_url).await?;
    let dialect = SqlDialect::from(&config.database_type);
    let report = validate_ingest(&pool, dialect, &args).await?;
    let output = render_validation_report(&report, args.json)?;

    println!("{output}");

    Ok(report.exit_code())
}
