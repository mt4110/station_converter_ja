use std::{process::ExitCode, str::FromStr};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    AnyPool, Row, SqlitePool,
};
use station_shared::{
    config::{AppConfig, DatabaseType},
    db::{connect_any_pool, decode_required_string, integer_aggregate_sql, SqlDialect},
};

const PARITY_FAILURE_EXIT_CODE: u8 = 2;
const SOURCE_NAME: &str = "ksj_n02_station";
const TABLES: [&str; 5] = [
    "source_snapshots",
    "station_identities",
    "station_versions",
    "station_change_events",
    "stations_latest",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParityStatus {
    Ok,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParityCheck {
    pub name: String,
    pub status: ParityStatus,
    pub source: serde_json::Value,
    pub sqlite: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParityReport {
    pub status: ParityStatus,
    pub sqlite_url: String,
    pub checks: Vec<ParityCheck>,
}

impl ParityReport {
    pub fn exit_code(&self) -> ExitCode {
        if self.status == ParityStatus::Failed {
            ExitCode::from(PARITY_FAILURE_EXIT_CODE)
        } else {
            ExitCode::SUCCESS
        }
    }
}

pub async fn verify_sqlite_parity(config: &AppConfig) -> Result<ParityReport> {
    if matches!(config.database_type, DatabaseType::Sqlite) {
        bail!("verify-sqlite-parity expects DATABASE_TYPE to be postgres or mysql");
    }

    let sqlite_url = sqlite_database_url();
    if sqlite_url_requests_memory(&sqlite_url) {
        bail!("SQLITE_DATABASE_URL must point to a file, not an in-memory database");
    }

    let source_pool = connect_any_pool(&config.database_url).await?;
    let source_dialect = SqlDialect::from(&config.database_type);
    let sqlite_pool = connect_sqlite_read_only(&sqlite_url).await?;

    let mut checks = Vec::new();
    for table in TABLES {
        let source_count = table_count(&source_pool, source_dialect, table).await?;
        let sqlite_count = sqlite_table_count(&sqlite_pool, table).await?;
        checks.push(ParityCheck {
            name: format!("{table}_count"),
            status: parity_status(source_count == sqlite_count),
            source: serde_json::json!(source_count),
            sqlite: serde_json::json!(sqlite_count),
        });
    }

    let source_latest = latest_source_snapshot(&source_pool, source_dialect).await?;
    let sqlite_latest = sqlite_latest_source_snapshot(&sqlite_pool).await?;
    checks.push(ParityCheck {
        name: "latest_source_snapshot".to_string(),
        status: parity_status(source_latest == sqlite_latest),
        source: serde_json::to_value(&source_latest)?,
        sqlite: serde_json::to_value(&sqlite_latest)?,
    });

    sqlite_pool.close().await;
    source_pool.close().await;

    Ok(ParityReport {
        status: if checks
            .iter()
            .any(|check| check.status == ParityStatus::Failed)
        {
            ParityStatus::Failed
        } else {
            ParityStatus::Ok
        },
        sqlite_url,
        checks,
    })
}

fn parity_status(matches: bool) -> ParityStatus {
    if matches {
        ParityStatus::Ok
    } else {
        ParityStatus::Failed
    }
}

fn sqlite_database_url() -> String {
    std::env::var("SQLITE_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://storage/sqlite/stations.sqlite3".to_string())
}

async fn connect_sqlite_read_only(sqlite_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(sqlite_url)
        .with_context(|| format!("unsupported SQLITE_DATABASE_URL: {sqlite_url}"))?
        .read_only(true)
        .foreign_keys(true);

    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?)
}

fn sqlite_url_requests_memory(sqlite_url: &str) -> bool {
    let trimmed = sqlite_url
        .trim_start_matches("sqlite://")
        .trim_start_matches("sqlite:");
    let mut parts = trimmed.splitn(2, '?');
    let database = parts.next().unwrap_or_default();

    if database == ":memory:" {
        return true;
    }

    parts.next().is_some_and(|query| {
        query.split('&').any(|param| {
            let mut pair = param.splitn(2, '=');
            matches!((pair.next(), pair.next()), (Some("mode"), Some("memory")))
        })
    })
}

async fn table_count(pool: &AnyPool, dialect: SqlDialect, table: &str) -> Result<i64> {
    let sql = format!(
        "SELECT {} AS count FROM {table}",
        integer_aggregate_sql(dialect, "COUNT(*)")
    );
    let row = sqlx::query(&dialect.statement(&sql))
        .fetch_one(pool)
        .await?;
    Ok(row.try_get("count")?)
}

async fn sqlite_table_count(pool: &SqlitePool, table: &str) -> Result<i64> {
    let sql = format!("SELECT CAST(COUNT(*) AS BIGINT) AS count FROM {table}");
    let row = sqlx::query(&sql).fetch_one(pool).await?;
    Ok(row.try_get("count")?)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct LatestSourceSnapshot {
    id: i64,
    source_sha256: String,
}

async fn latest_source_snapshot(
    pool: &AnyPool,
    dialect: SqlDialect,
) -> Result<Option<LatestSourceSnapshot>> {
    let row = sqlx::query(&dialect.statement(
        "SELECT id, source_sha256
         FROM source_snapshots
         WHERE source_name = ?
         ORDER BY id DESC
         LIMIT 1",
    ))
    .bind(SOURCE_NAME)
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        Ok(LatestSourceSnapshot {
            id: row.try_get("id")?,
            source_sha256: decode_required_string(&row, "source_sha256")?,
        })
    })
    .transpose()
}

async fn sqlite_latest_source_snapshot(pool: &SqlitePool) -> Result<Option<LatestSourceSnapshot>> {
    let row = sqlx::query(
        "SELECT id, source_sha256
         FROM source_snapshots
         WHERE source_name = ?
         ORDER BY id DESC
         LIMIT 1",
    )
    .bind(SOURCE_NAME)
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        Ok(LatestSourceSnapshot {
            id: row.try_get("id")?,
            source_sha256: row.try_get("source_sha256")?,
        })
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_sqlite_urls_are_rejected() {
        assert!(sqlite_url_requests_memory("sqlite::memory:"));
        assert!(sqlite_url_requests_memory("sqlite://fixture?mode=memory"));
        assert!(!sqlite_url_requests_memory(
            "sqlite://storage/sqlite/stations.sqlite3"
        ));
    }
}
