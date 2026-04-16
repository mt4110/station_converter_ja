use anyhow::Result;
use clap::{Parser, Subcommand};
use sqlx::{
    migrate::Migrator, mysql::MySqlPoolOptions, postgres::PgPoolOptions, sqlite::SqlitePoolOptions,
};
use station_shared::config::{AppConfig, DatabaseType};
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
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let cli = Cli::parse();
    let config = AppConfig::from_env("station-ops")?;

    match cli.command {
        Commands::Migrate => migrate(&config).await?,
        Commands::ExportSqlite => {
            info!("export-sqlite is a reserved command. Real export is not implemented in this scaffold.");
        }
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
            let pool = SqlitePoolOptions::new()
                .connect(&config.database_url)
                .await?;
            SQLITE_MIGRATOR.run(&pool).await?;
        }
    }

    info!("migrations complete for {}", config.database_type);
    Ok(())
}
