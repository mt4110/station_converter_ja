use anyhow::{anyhow, Result};
use std::{env, fmt::Display, str::FromStr};

#[derive(Clone, Debug)]
pub enum DatabaseType {
    Postgres,
    Mysql,
    Sqlite,
}

impl DatabaseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::Mysql => "mysql",
            Self::Sqlite => "sqlite",
        }
    }
}

impl Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DatabaseType {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "postgres" => Ok(Self::Postgres),
            "mysql" => Ok(Self::Mysql),
            "sqlite" => Ok(Self::Sqlite),
            other => Err(anyhow!("unsupported DATABASE_TYPE: {other}")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub service_name: String,
    pub bind_addr: String,
    pub database_type: DatabaseType,
    pub database_url: String,
    pub redis_url: Option<String>,
    pub ready_require_cache: bool,
    pub update_interval_seconds: u64,
    pub source_snapshot_url: Option<String>,
    pub allow_local_source_snapshot: bool,
    pub temp_asset_dir: String,
    pub ingest_write_chunk_size: usize,
    pub ingest_close_chunk_size: usize,
}

impl AppConfig {
    pub fn from_env(service_name: &str) -> Result<Self> {
        let database_type = env::var("DATABASE_TYPE")
            .unwrap_or_else(|_| "postgres".to_string())
            .parse::<DatabaseType>()?;

        let database_url = match database_type {
            DatabaseType::Postgres => env::var("POSTGRES_DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres_password@127.0.0.1:3215/station_db".to_string()
            }),
            DatabaseType::Mysql => env::var("MYSQL_DATABASE_URL").unwrap_or_else(|_| {
                "mysql://station_user:station_password@127.0.0.1:3214/station_db".to_string()
            }),
            DatabaseType::Sqlite => env::var("SQLITE_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://storage/sqlite/stations.sqlite3".to_string()),
        };

        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3212".to_string());
        let redis_url = env::var("REDIS_URL").ok().filter(|v| !v.is_empty());
        let ready_require_cache = env::var("READY_REQUIRE_CACHE")
            .unwrap_or_else(|_| "false".to_string())
            .eq_ignore_ascii_case("true");
        let update_interval_seconds = env::var("UPDATE_INTERVAL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(86400);

        let source_snapshot_url = env::var("SOURCE_SNAPSHOT_URL")
            .ok()
            .filter(|v| !v.is_empty());
        let allow_local_source_snapshot = env::var("ALLOW_LOCAL_SOURCE_SNAPSHOT")
            .unwrap_or_else(|_| "false".to_string())
            .eq_ignore_ascii_case("true");
        let temp_asset_dir =
            env::var("TEMP_ASSET_DIR").unwrap_or_else(|_| "worker/crawler/temp_assets".to_string());
        let ingest_write_chunk_size = match env_usize_optional("INGEST_WRITE_CHUNK_SIZE")? {
            Some(value) => value,
            None => default_ingest_write_chunk_size(&database_type),
        };
        let ingest_close_chunk_size =
            env_usize_optional("INGEST_CLOSE_CHUNK_SIZE")?.unwrap_or(1000);

        Ok(Self {
            service_name: service_name.to_string(),
            bind_addr,
            database_type,
            database_url,
            redis_url,
            ready_require_cache,
            update_interval_seconds,
            source_snapshot_url,
            allow_local_source_snapshot,
            temp_asset_dir,
            ingest_write_chunk_size,
            ingest_close_chunk_size,
        })
    }
}

fn env_usize_optional(name: &str) -> Result<Option<usize>> {
    let value = match env::var(name) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let parsed = value
        .parse::<usize>()
        .map_err(|_| anyhow!("{name} must be a positive integer"))?;

    if parsed == 0 {
        return Err(anyhow!("{name} must be greater than 0"));
    }

    Ok(Some(parsed))
}

fn default_ingest_write_chunk_size(database_type: &DatabaseType) -> usize {
    match database_type {
        DatabaseType::Postgres | DatabaseType::Sqlite => 1000,
        DatabaseType::Mysql => 200,
    }
}
