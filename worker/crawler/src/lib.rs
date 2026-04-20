pub mod n02;

use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use sqlx::AnyPool;
use station_shared::{config::AppConfig, db::SqlDialect};
use tracing::info;

pub use n02::{IngestReport, PersistChunkConfig};

pub const DEFAULT_SOURCE_SNAPSHOT_URL: &str =
    "https://nlftp.mlit.go.jp/ksj/gml/data/N02/N02-24/N02-24_GML.zip";
pub const N02_INGEST_LOCK_NAME: &str = "ingest-n02";

pub async fn run_n02_ingest_cycle(
    config: &AppConfig,
    pool: &AnyPool,
    dialect: SqlDialect,
) -> Result<IngestReport> {
    let total_start = Instant::now();
    let source_url = config
        .source_snapshot_url
        .as_deref()
        .unwrap_or(DEFAULT_SOURCE_SNAPSHOT_URL);

    if !is_remote_snapshot_url(source_url) && !config.allow_local_source_snapshot {
        bail!(
            "local snapshot URLs are disabled by default; set ALLOW_LOCAL_SOURCE_SNAPSHOT=true when you intentionally want fixture/file ingest"
        );
    }

    tokio::fs::create_dir_all(&config.temp_asset_dir).await?;

    let load_start = Instant::now();
    let bytes = load_snapshot_bytes(source_url).await?;
    let load_ms = duration_ms(load_start.elapsed());

    let filename = format!("snapshot-{}.zip", chrono::Utc::now().format("%Y%m%d%H%M%S"));
    let output_path = format!("{}/{}", config.temp_asset_dir, filename);

    let save_start = Instant::now();
    tokio::fs::write(&output_path, &bytes).await?;
    let save_zip_ms = duration_ms(save_start.elapsed());

    let mut report = n02::ingest_snapshot_with_config(
        pool,
        dialect,
        source_url,
        &output_path,
        bytes.as_ref(),
        PersistChunkConfig {
            write_chunk_size: config.ingest_write_chunk_size,
            close_chunk_size: config.ingest_close_chunk_size,
        },
    )
    .await?;
    report.load_ms = load_ms;
    report.save_zip_ms = save_zip_ms;
    report.total_ms = duration_ms(total_start.elapsed());

    Ok(report)
}

fn is_remote_snapshot_url(url: &str) -> bool {
    let Some((scheme, _)) = url.split_once("://") else {
        return false;
    };

    scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http")
}

async fn load_snapshot_bytes(source_url: &str) -> Result<Vec<u8>> {
    if is_remote_snapshot_url(source_url) {
        info!(source_url = %source_url, "downloading source snapshot");
        let response = reqwest::get(source_url).await?.error_for_status()?;
        let bytes = response.bytes().await?;
        return Ok(bytes.to_vec());
    }

    let path = source_url.strip_prefix("file://").unwrap_or(source_url);
    info!(path, "loading local source snapshot");
    tokio::fs::read(path)
        .await
        .with_context(|| format!("failed to read local snapshot: {path}"))
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Cursor, Write},
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use sqlx::any::AnyPoolOptions;
    use station_shared::config::DatabaseType;
    use station_shared::db::ensure_sqlx_drivers;
    use zip::{write::SimpleFileOptions, ZipWriter};

    use super::*;

    #[test]
    fn remote_snapshot_url_check_is_case_insensitive() {
        assert!(is_remote_snapshot_url("HTTPS://example.com/N02.zip"));
        assert!(is_remote_snapshot_url("http://example.com/N02.zip"));
        assert!(!is_remote_snapshot_url("/tmp/N02.zip"));
    }

    #[tokio::test]
    async fn run_n02_ingest_cycle_reports_phase_timings() {
        let pool = test_pool().await;
        let test_dir = unique_test_dir("station-crawler-phase-timing");
        let output_dir = test_dir.join("output");
        let snapshot_path = test_dir.join("fixtures").join("N02-24_GML.zip");

        std::fs::create_dir_all(snapshot_path.parent().unwrap()).unwrap();
        std::fs::write(&snapshot_path, snapshot_zip_bytes(sample_geojson())).unwrap();

        let config = AppConfig {
            service_name: "station-crawler".to_string(),
            bind_addr: "127.0.0.1:0".to_string(),
            database_type: DatabaseType::Sqlite,
            database_url: "sqlite::memory:".to_string(),
            job_lock_dir: test_dir.join("locks").display().to_string(),
            redis_url: None,
            ready_require_cache: false,
            update_interval_seconds: 60,
            source_snapshot_url: Some(format!("file://{}", snapshot_path.display())),
            allow_local_source_snapshot: true,
            temp_asset_dir: output_dir.display().to_string(),
            ingest_write_chunk_size: 1000,
            ingest_close_chunk_size: 1000,
        };

        let report = run_n02_ingest_cycle(&config, &pool, SqlDialect::Sqlite)
            .await
            .unwrap();

        assert_eq!(report.created, 2);
        assert!(output_dir.exists());
        assert_full_cycle_phase_timings_are_sane(&report);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    fn assert_full_cycle_phase_timings_are_sane(report: &IngestReport) {
        let report_json = serde_json::to_value(report).unwrap();
        let total_ms = phase_timing_value(&report_json, "total_ms");
        let component_keys = [
            "load_ms",
            "save_zip_ms",
            "extract_ms",
            "parse_ms",
            "diff_ms",
            "persist_ms",
        ];
        let component_sum = component_keys
            .iter()
            .map(|key| {
                let value = phase_timing_value(&report_json, key);
                assert!(value <= total_ms, "{key} should not exceed total_ms");
                value
            })
            .sum::<u64>();

        assert!(
            total_ms >= component_sum,
            "total_ms should cover all phase timings"
        );
    }

    fn phase_timing_value(report_json: &serde_json::Value, key: &str) -> u64 {
        report_json
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| panic!("{key} should be present in serialized ingest report"))
    }

    async fn test_pool() -> AnyPool {
        ensure_sqlx_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        apply_sqlite_schema(&pool).await;
        pool
    }

    async fn apply_sqlite_schema(pool: &AnyPool) {
        for statement in include_str!("../../../storage/migrations/sqlite/0001_init.sql")
            .split(';')
            .map(str::trim)
            .filter(|statement| !statement.is_empty())
        {
            sqlx::query(statement).execute(pool).await.unwrap();
        }
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{suffix}"))
    }

    fn sample_geojson() -> &'static str {
        r#"{
          "features": [
            {
              "properties": {
                "N02_003": "京王線",
                "N02_004": "京王電鉄",
                "N02_005": "新宿",
                "N02_005c": "003700",
                "N02_005g": "003700"
              },
              "geometry": {
                "type": "LineString",
                "coordinates": [[139.699, 35.690], [139.701, 35.692]]
              }
            },
            {
              "properties": {
                "N02_003": "中央線",
                "N02_004": "東日本旅客鉄道",
                "N02_005": "中野",
                "N02_005c": "003568",
                "N02_005g": "003568"
              },
              "geometry": {
                "type": "LineString",
                "coordinates": [[139.665, 35.705], [139.666, 35.706]]
              }
            }
          ]
        }"#
    }

    fn snapshot_zip_bytes(geojson: &str) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("UTF-8/N02-24_Station.geojson", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(geojson.as_bytes()).unwrap();
        writer.finish().unwrap().into_inner()
    }
}
