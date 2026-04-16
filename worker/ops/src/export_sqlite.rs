use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use futures_util::TryStreamExt;
use sqlx::{
    any::AnyRow,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, Sqlite, Transaction,
};
use station_shared::{
    config::{AppConfig, DatabaseType},
    db::{connect_any_pool, SqlDialect},
};
use tokio::fs;
use tracing::{info, warn};

static SQLITE_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../storage/migrations/sqlite");

pub struct ExportReport {
    pub output_path: PathBuf,
    pub source_snapshots: usize,
    pub station_identities: usize,
    pub station_versions: usize,
    pub station_change_events: usize,
}

pub async fn export_sqlite(config: &AppConfig) -> Result<ExportReport> {
    if matches!(config.database_type, DatabaseType::Sqlite) {
        bail!("export-sqlite expects DATABASE_TYPE to be postgres or mysql");
    }

    let sqlite_url = sqlite_database_url();
    let output_path = sqlite_url_to_path(&sqlite_url)?;
    let temp_path = temp_output_path(&output_path);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    if fs::try_exists(&temp_path).await? {
        fs::remove_file(&temp_path).await?;
    }

    let source_pool = connect_any_pool(&config.database_url).await?;
    let source_dialect = SqlDialect::from(&config.database_type);

    let sqlite_options = SqliteConnectOptions::new()
        .filename(&temp_path)
        .create_if_missing(true)
        .foreign_keys(true);
    let sqlite_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_options)
        .await?;

    SQLITE_MIGRATOR.run(&sqlite_pool).await?;

    let mut tx = sqlite_pool.begin().await?;
    let source_snapshots = copy_source_snapshots(&source_pool, source_dialect, &mut tx).await?;
    let station_identities = copy_station_identities(&source_pool, source_dialect, &mut tx).await?;
    let station_versions = copy_station_versions(&source_pool, source_dialect, &mut tx).await?;
    let station_change_events =
        copy_station_change_events(&source_pool, source_dialect, &mut tx).await?;
    tx.commit().await?;

    sqlx::query("PRAGMA optimize").execute(&sqlite_pool).await?;
    sqlx::query("VACUUM").execute(&sqlite_pool).await?;
    sqlite_pool.close().await;

    install_output_file(&temp_path, &output_path).await?;

    let report = ExportReport {
        output_path,
        source_snapshots,
        station_identities,
        station_versions,
        station_change_events,
    };

    info!(
        output_path = %report.output_path.display(),
        source_snapshots = report.source_snapshots,
        station_identities = report.station_identities,
        station_versions = report.station_versions,
        station_change_events = report.station_change_events,
        "sqlite export complete"
    );

    Ok(report)
}

fn sqlite_database_url() -> String {
    std::env::var("SQLITE_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://storage/sqlite/stations.sqlite3".to_string())
}

fn sqlite_url_to_path(sqlite_url: &str) -> Result<PathBuf> {
    if sqlite_url_requests_memory(sqlite_url) {
        bail!("SQLITE_DATABASE_URL must point to a file, not an in-memory database");
    }

    let options = SqliteConnectOptions::from_str(sqlite_url)
        .with_context(|| format!("unsupported SQLITE_DATABASE_URL: {sqlite_url}"))?;

    Ok(options.get_filename().to_path_buf())
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

fn temp_output_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "stations.sqlite3".to_string());

    path.with_file_name(format!("{file_name}.tmp"))
}

fn backup_output_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "stations.sqlite3".to_string());

    path.with_file_name(format!("{file_name}.bak"))
}

async fn install_output_file(temp_path: &Path, output_path: &Path) -> Result<()> {
    let backup_path = backup_output_path(output_path);

    if fs::try_exists(&backup_path).await? {
        fs::remove_file(&backup_path).await.with_context(|| {
            format!(
                "failed to clear stale sqlite backup {}",
                backup_path.display()
            )
        })?;
    }

    if !fs::try_exists(output_path).await? {
        fs::rename(temp_path, output_path).await.with_context(|| {
            format!("failed to place sqlite artifact {}", output_path.display())
        })?;
        return Ok(());
    }

    fs::rename(output_path, &backup_path)
        .await
        .with_context(|| {
            format!(
                "failed to move existing sqlite artifact {} to backup {}",
                output_path.display(),
                backup_path.display()
            )
        })?;

    match fs::rename(temp_path, output_path).await {
        Ok(()) => {
            if let Err(err) = fs::remove_file(&backup_path).await {
                warn!(
                    backup_path = %backup_path.display(),
                    error = %err,
                    "failed to remove sqlite artifact backup"
                );
            }

            Ok(())
        }
        Err(err) => {
            fs::rename(&backup_path, output_path).await.with_context(|| {
                format!(
                    "failed to replace sqlite artifact {}; original artifact could not be restored from {}",
                    output_path.display(),
                    backup_path.display()
                )
            })?;

            Err(err).with_context(|| {
                format!(
                    "failed to replace sqlite artifact {}",
                    output_path.display()
                )
            })
        }
    }
}

fn text_select(dialect: SqlDialect, column: &str) -> String {
    format!("{} AS {column}", dialect.text_cast(column))
}

fn row_string(row: &AnyRow, column: &str) -> Result<String> {
    match row.try_get::<String, _>(column) {
        Ok(value) => Ok(value),
        Err(_) => {
            let bytes = row.try_get::<Vec<u8>, _>(column)?;
            String::from_utf8(bytes)
                .with_context(|| format!("column '{column}' contained non-utf8 bytes"))
        }
    }
}

fn row_optional_string(row: &AnyRow, column: &str) -> Result<Option<String>> {
    match row.try_get::<Option<String>, _>(column) {
        Ok(value) => Ok(value),
        Err(_) => row
            .try_get::<Option<Vec<u8>>, _>(column)?
            .map(|bytes| {
                String::from_utf8(bytes)
                    .with_context(|| format!("column '{column}' contained non-utf8 bytes"))
            })
            .transpose(),
    }
}

async fn copy_source_snapshots(
    source_pool: &sqlx::AnyPool,
    source_dialect: SqlDialect,
    tx: &mut Transaction<'_, Sqlite>,
) -> Result<usize> {
    let query = format!(
        "SELECT
           id,
           {},
           {},
           {},
           {},
           {},
           {} AS downloaded_at
         FROM source_snapshots
         ORDER BY id",
        text_select(source_dialect, "source_name"),
        text_select(source_dialect, "source_kind"),
        text_select(source_dialect, "source_version"),
        text_select(source_dialect, "source_url"),
        text_select(source_dialect, "source_sha256"),
        source_dialect.text_cast("downloaded_at")
    );

    let mut rows = sqlx::query(&query).fetch(source_pool);
    let mut count = 0;

    while let Some(row) = rows.try_next().await? {
        sqlx::query(
            "INSERT INTO source_snapshots (
               id,
               source_name,
               source_kind,
               source_version,
               source_url,
               source_sha256,
               downloaded_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(row.try_get::<i64, _>("id")?)
        .bind(row_string(&row, "source_name")?)
        .bind(row_string(&row, "source_kind")?)
        .bind(row_optional_string(&row, "source_version")?)
        .bind(row_string(&row, "source_url")?)
        .bind(row_string(&row, "source_sha256")?)
        .bind(row_string(&row, "downloaded_at")?)
        .execute(&mut **tx)
        .await?;

        count += 1;
    }

    Ok(count)
}

async fn copy_station_identities(
    source_pool: &sqlx::AnyPool,
    source_dialect: SqlDialect,
    tx: &mut Transaction<'_, Sqlite>,
) -> Result<usize> {
    let query = format!(
        "SELECT
           id,
           {},
           {},
           {} AS created_at
         FROM station_identities
         ORDER BY id",
        text_select(source_dialect, "station_uid"),
        text_select(source_dialect, "canonical_name"),
        source_dialect.text_cast("created_at")
    );

    let mut rows = sqlx::query(&query).fetch(source_pool);
    let mut count = 0;

    while let Some(row) = rows.try_next().await? {
        sqlx::query(
            "INSERT INTO station_identities (
               id,
               station_uid,
               canonical_name,
               created_at
             ) VALUES (?, ?, ?, ?)",
        )
        .bind(row.try_get::<i64, _>("id")?)
        .bind(row_string(&row, "station_uid")?)
        .bind(row_string(&row, "canonical_name")?)
        .bind(row_string(&row, "created_at")?)
        .execute(&mut **tx)
        .await?;

        count += 1;
    }

    Ok(count)
}

async fn copy_station_versions(
    source_pool: &sqlx::AnyPool,
    source_dialect: SqlDialect,
    tx: &mut Transaction<'_, Sqlite>,
) -> Result<usize> {
    let query = format!(
        "SELECT
           id,
           {},
           snapshot_id,
           {},
           {},
           {},
           {},
           {},
           latitude,
           longitude,
           {},
           {},
           {} AS opened_on,
           {} AS closed_on,
           {} AS valid_from,
           {} AS valid_to,
           {}
         FROM station_versions
         ORDER BY id",
        text_select(source_dialect, "station_uid"),
        text_select(source_dialect, "source_station_code"),
        text_select(source_dialect, "source_group_code"),
        text_select(source_dialect, "station_name"),
        text_select(source_dialect, "line_name"),
        text_select(source_dialect, "operator_name"),
        text_select(source_dialect, "geometry_geojson"),
        text_select(source_dialect, "status"),
        text_select(source_dialect, "opened_on"),
        text_select(source_dialect, "closed_on"),
        text_select(source_dialect, "valid_from"),
        text_select(source_dialect, "valid_to"),
        text_select(source_dialect, "change_hash")
    );

    let mut rows = sqlx::query(&query).fetch(source_pool);
    let mut count = 0;

    while let Some(row) = rows.try_next().await? {
        sqlx::query(
            "INSERT INTO station_versions (
               id,
               station_uid,
               snapshot_id,
               source_station_code,
               source_group_code,
               station_name,
               line_name,
               operator_name,
               latitude,
               longitude,
               geometry_geojson,
               status,
               opened_on,
               closed_on,
               valid_from,
               valid_to,
               change_hash
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(row.try_get::<i64, _>("id")?)
        .bind(row_string(&row, "station_uid")?)
        .bind(row.try_get::<i64, _>("snapshot_id")?)
        .bind(row_optional_string(&row, "source_station_code")?)
        .bind(row_optional_string(&row, "source_group_code")?)
        .bind(row_string(&row, "station_name")?)
        .bind(row_string(&row, "line_name")?)
        .bind(row_string(&row, "operator_name")?)
        .bind(row.try_get::<f64, _>("latitude")?)
        .bind(row.try_get::<f64, _>("longitude")?)
        .bind(row_optional_string(&row, "geometry_geojson")?)
        .bind(row_string(&row, "status")?)
        .bind(row_optional_string(&row, "opened_on")?)
        .bind(row_optional_string(&row, "closed_on")?)
        .bind(row_string(&row, "valid_from")?)
        .bind(row_optional_string(&row, "valid_to")?)
        .bind(row_string(&row, "change_hash")?)
        .execute(&mut **tx)
        .await?;

        count += 1;
    }

    Ok(count)
}

async fn copy_station_change_events(
    source_pool: &sqlx::AnyPool,
    source_dialect: SqlDialect,
    tx: &mut Transaction<'_, Sqlite>,
) -> Result<usize> {
    let query = format!(
        "SELECT
           id,
           snapshot_id,
           {},
           {},
           before_version_id,
           after_version_id,
           {},
           {} AS created_at
         FROM station_change_events
         ORDER BY id",
        text_select(source_dialect, "station_uid"),
        text_select(source_dialect, "change_kind"),
        text_select(source_dialect, "detail_json"),
        source_dialect.text_cast("created_at")
    );

    let mut rows = sqlx::query(&query).fetch(source_pool);
    let mut count = 0;

    while let Some(row) = rows.try_next().await? {
        sqlx::query(
            "INSERT INTO station_change_events (
               id,
               snapshot_id,
               station_uid,
               change_kind,
               before_version_id,
               after_version_id,
               detail_json,
               created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(row.try_get::<i64, _>("id")?)
        .bind(row.try_get::<i64, _>("snapshot_id")?)
        .bind(row_string(&row, "station_uid")?)
        .bind(row_string(&row, "change_kind")?)
        .bind(row.try_get::<Option<i64>, _>("before_version_id")?)
        .bind(row.try_get::<Option<i64>, _>("after_version_id")?)
        .bind(row_optional_string(&row, "detail_json")?)
        .bind(row_string(&row, "created_at")?)
        .execute(&mut **tx)
        .await?;

        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn parses_relative_sqlite_url() {
        let path = sqlite_url_to_path("sqlite://storage/sqlite/stations.sqlite3").unwrap();
        assert_eq!(path, PathBuf::from("storage/sqlite/stations.sqlite3"));
    }

    #[test]
    fn parses_absolute_sqlite_url() {
        let path = sqlite_url_to_path(
            "sqlite:///opt/station_converter_ja/storage/sqlite/stations.sqlite3",
        )
        .unwrap();
        assert_eq!(
            path,
            PathBuf::from("/opt/station_converter_ja/storage/sqlite/stations.sqlite3")
        );
    }

    #[test]
    fn ignores_sqlite_url_query_when_deriving_output_path() {
        let path = sqlite_url_to_path(
            "sqlite:///opt/station_converter_ja/storage/sqlite/stations.sqlite3?mode=rwc",
        )
        .unwrap();
        assert_eq!(
            path,
            PathBuf::from("/opt/station_converter_ja/storage/sqlite/stations.sqlite3")
        );
    }

    #[test]
    fn rejects_in_memory_sqlite_url() {
        let err = sqlite_url_to_path("sqlite::memory:").expect_err("in-memory URL should fail");
        assert!(err.to_string().contains("must point to a file"));
    }

    #[test]
    fn rejects_memory_mode_sqlite_url() {
        let err = sqlite_url_to_path("sqlite://artifact.db?mode=memory")
            .expect_err("memory mode URL should fail");
        assert!(err.to_string().contains("must point to a file"));
    }

    #[test]
    fn creates_temp_path_next_to_output() {
        let path = temp_output_path(Path::new("storage/sqlite/stations.sqlite3"));
        assert_eq!(path, PathBuf::from("storage/sqlite/stations.sqlite3.tmp"));
    }

    #[test]
    fn creates_backup_path_next_to_output() {
        let path = backup_output_path(Path::new("storage/sqlite/stations.sqlite3"));
        assert_eq!(path, PathBuf::from("storage/sqlite/stations.sqlite3.bak"));
    }

    #[test]
    fn mysql_text_select_casts_to_char() {
        assert_eq!(
            text_select(SqlDialect::Mysql, "detail_json"),
            "CAST(detail_json AS CHAR) AS detail_json"
        );
    }

    #[test]
    fn optional_string_from_none_bytes_is_none() {
        let none_bytes: Option<Vec<u8>> = None;
        assert!(none_bytes
            .map(|bytes| {
                String::from_utf8(bytes)
                    .with_context(|| "column 'detail_json' contained non-utf8 bytes")
            })
            .transpose()
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn install_output_file_replaces_existing_output() -> Result<()> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let dir = std::env::temp_dir().join(format!("station-export-install-{unique}"));
        fs::create_dir_all(&dir).await?;

        let output_path = dir.join("stations.sqlite3");
        let temp_path = dir.join("stations.sqlite3.tmp");
        let backup_path = dir.join("stations.sqlite3.bak");

        fs::write(&output_path, b"old").await?;
        fs::write(&temp_path, b"new").await?;

        install_output_file(&temp_path, &output_path).await?;

        assert_eq!(fs::read(&output_path).await?, b"new");
        assert!(!fs::try_exists(&temp_path).await?);
        assert!(!fs::try_exists(&backup_path).await?);

        fs::remove_dir_all(dir).await?;

        Ok(())
    }
}
