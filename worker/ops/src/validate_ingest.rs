use std::{fmt::Write, process::ExitCode};

use anyhow::Result;
use clap::Args;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{any::AnyRow, AnyPool, Row};
use station_shared::db::SqlDialect;

const SOURCE_NAME: &str = "ksj_n02_station";
const STATION_UID_LIKE_PATTERN: &str = "stn_n02_%";

const DEFAULT_MIN_STATIONS: i64 = 10_000;
const DEFAULT_MIN_LINES: i64 = 500;
const DEFAULT_MIN_OPERATORS: i64 = 150;

const HARD_MIN_LATITUDE: f64 = 20.0;
const HARD_MAX_LATITUDE: f64 = 46.5;
const HARD_MIN_LONGITUDE: f64 = 122.0;
const HARD_MAX_LONGITUDE: f64 = 154.0;

const WARN_MIN_LATITUDE: f64 = 24.0;
const WARN_MAX_LATITUDE: f64 = 46.0;
const WARN_MIN_LONGITUDE: f64 = 123.0;
const WARN_MAX_LONGITUDE: f64 = 146.0;

const VALIDATION_FAILURE_EXIT_CODE: u8 = 2;

#[derive(Debug, Clone, Args)]
pub struct ValidateIngestArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub strict: bool,
    #[arg(long, default_value_t = DEFAULT_MIN_STATIONS)]
    pub min_stations: i64,
    #[arg(long, default_value_t = DEFAULT_MIN_LINES)]
    pub min_lines: i64,
    #[arg(long, default_value_t = DEFAULT_MIN_OPERATORS)]
    pub min_operators: i64,
}

impl Default for ValidateIngestArgs {
    fn default() -> Self {
        Self {
            json: false,
            strict: false,
            min_stations: DEFAULT_MIN_STATIONS,
            min_lines: DEFAULT_MIN_LINES,
            min_operators: DEFAULT_MIN_OPERATORS,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Ok,
    Warning,
    Failed,
}

impl ValidationStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationCheck {
    pub name: &'static str,
    pub status: ValidationStatus,
    pub observed: Value,
    pub expected: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub status: ValidationStatus,
    pub strict: bool,
    pub snapshot_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub checks: Vec<ValidationCheck>,
}

impl ValidationReport {
    pub fn exit_code(&self) -> ExitCode {
        if self.status == ValidationStatus::Failed {
            ExitCode::from(VALIDATION_FAILURE_EXIT_CODE)
        } else {
            ExitCode::SUCCESS
        }
    }
}

#[derive(Debug)]
struct SnapshotMeta {
    id: i64,
    source_version: Option<String>,
    source_url: String,
}

#[derive(Debug)]
struct LatestMetrics {
    station_count: i64,
    line_count: i64,
    operator_count: i64,
    blank_station_name_count: i64,
    blank_line_name_count: i64,
    blank_operator_name_count: i64,
    out_of_range_coordinate_count: i64,
    suspicious_coordinate_count: i64,
}

#[derive(Debug)]
struct ChangeSummary {
    created_count: i64,
    updated_count: i64,
    removed_count: i64,
    latest_snapshot_version_count: i64,
}

pub async fn validate_ingest(
    pool: &AnyPool,
    dialect: SqlDialect,
    args: &ValidateIngestArgs,
) -> Result<ValidationReport> {
    let mut report = ValidationReport {
        status: ValidationStatus::Ok,
        strict: args.strict,
        snapshot_id: None,
        source_version: None,
        source_url: None,
        checks: Vec::new(),
    };

    let latest_snapshot = fetch_latest_snapshot(pool, dialect).await?;
    match latest_snapshot {
        None => {
            report.checks.push(failed_check(
                "latest_snapshot_present",
                Value::Null,
                json!("present"),
                Some("N02 source snapshot is missing".to_string()),
            ));
            report.status = ValidationStatus::Failed;
            Ok(report)
        }
        Some(snapshot) => {
            report.snapshot_id = Some(snapshot.id);
            report.source_version = snapshot.source_version.clone();
            report.source_url = Some(snapshot.source_url.clone());
            report.checks.push(ok_check(
                "latest_snapshot_present",
                json!(snapshot.id),
                json!("present"),
                None,
            ));

            report
                .checks
                .push(source_url_quality_check(&snapshot.source_url));

            let metrics = fetch_latest_metrics(pool, dialect).await?;
            report.checks.push(threshold_check(
                "min_station_count",
                metrics.station_count,
                args.min_stations,
            ));
            report.checks.push(threshold_check(
                "min_line_count",
                metrics.line_count,
                args.min_lines,
            ));
            report.checks.push(threshold_check(
                "min_operator_count",
                metrics.operator_count,
                args.min_operators,
            ));
            report.checks.push(zero_check(
                "blank_station_name_count",
                metrics.blank_station_name_count,
            ));
            report.checks.push(zero_check(
                "blank_line_name_count",
                metrics.blank_line_name_count,
            ));
            report.checks.push(zero_check(
                "blank_operator_name_count",
                metrics.blank_operator_name_count,
            ));
            report.checks.push(zero_check(
                "out_of_range_coordinate_count",
                metrics.out_of_range_coordinate_count,
            ));
            report.checks.push(suspicious_coordinate_check(
                metrics.suspicious_coordinate_count,
            ));

            let duplicate_station_uid_count =
                fetch_duplicate_station_uid_count(pool, dialect).await?;
            report.checks.push(zero_check(
                "duplicate_latest_station_uid_count",
                duplicate_station_uid_count,
            ));

            let change_summary = fetch_change_summary(pool, dialect, snapshot.id).await?;
            report
                .checks
                .push(change_summary_check(metrics.station_count, &change_summary));

            report.status = overall_status(&report.checks, args.strict);
            Ok(report)
        }
    }
}

pub fn render_validation_report(report: &ValidationReport, json_output: bool) -> Result<String> {
    if json_output {
        return Ok(serde_json::to_string_pretty(report)?);
    }

    let mut output = String::new();
    writeln!(&mut output, "validate-ingest: {}", report.status.label())?;
    writeln!(&mut output, "strict: {}", report.strict)?;
    if let Some(snapshot_id) = report.snapshot_id {
        writeln!(&mut output, "snapshot_id: {snapshot_id}")?;
    }
    if let Some(source_version) = &report.source_version {
        writeln!(&mut output, "source_version: {source_version}")?;
    }
    if let Some(source_url) = &report.source_url {
        writeln!(&mut output, "source_url: {source_url}")?;
    }

    for check in &report.checks {
        writeln!(
            &mut output,
            "- [{}] {} observed={} expected={}",
            check.status.label(),
            check.name,
            format_value(&check.observed),
            format_value(&check.expected)
        )?;

        if let Some(message) = &check.message {
            writeln!(&mut output, "  {}", message)?;
        }
    }

    Ok(output.trim_end().to_string())
}

async fn fetch_latest_snapshot(
    pool: &AnyPool,
    dialect: SqlDialect,
) -> Result<Option<SnapshotMeta>> {
    let row = sqlx::query(&dialect.statement(
        "SELECT id, source_version, source_url
         FROM source_snapshots
         WHERE source_name = ?
         ORDER BY id DESC
         LIMIT 1",
    ))
    .bind(SOURCE_NAME)
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        Ok(SnapshotMeta {
            id: row.try_get::<i64, _>("id")?,
            source_version: decode_optional_string(&row, "source_version")?,
            source_url: decode_required_string(&row, "source_url")?,
        })
    })
    .transpose()
}

async fn fetch_latest_metrics(pool: &AnyPool, dialect: SqlDialect) -> Result<LatestMetrics> {
    let sql = format!(
        "SELECT
           {} AS station_count,
           {} AS line_count,
           {} AS operator_count,
           {} AS blank_station_name_count,
           {} AS blank_line_name_count,
           {} AS blank_operator_name_count,
           {} AS out_of_range_coordinate_count,
           {} AS suspicious_coordinate_count
         FROM stations_latest
         WHERE station_uid LIKE ?",
        integer_aggregate_sql(dialect, "COUNT(*)"),
        integer_aggregate_sql(dialect, &distinct_text_count_sql(dialect, "line_name")),
        integer_aggregate_sql(dialect, &distinct_text_count_sql(dialect, "operator_name")),
        integer_aggregate_sql(
            dialect,
            "SUM(CASE WHEN TRIM(COALESCE(station_name, '')) = '' THEN 1 ELSE 0 END)"
        ),
        integer_aggregate_sql(
            dialect,
            "SUM(CASE WHEN TRIM(COALESCE(line_name, '')) = '' THEN 1 ELSE 0 END)"
        ),
        integer_aggregate_sql(
            dialect,
            "SUM(CASE WHEN TRIM(COALESCE(operator_name, '')) = '' THEN 1 ELSE 0 END)"
        ),
        integer_aggregate_sql(
            dialect,
            "SUM(CASE WHEN latitude < ? OR latitude > ? OR longitude < ? OR longitude > ? THEN 1 ELSE 0 END)"
        ),
        integer_aggregate_sql(
            dialect,
            "SUM(CASE WHEN latitude < ? OR latitude > ? OR longitude < ? OR longitude > ? THEN 1 ELSE 0 END)"
        ),
    );
    let row = sqlx::query(&dialect.statement(&sql))
        .bind(HARD_MIN_LATITUDE)
        .bind(HARD_MAX_LATITUDE)
        .bind(HARD_MIN_LONGITUDE)
        .bind(HARD_MAX_LONGITUDE)
        .bind(WARN_MIN_LATITUDE)
        .bind(WARN_MAX_LATITUDE)
        .bind(WARN_MIN_LONGITUDE)
        .bind(WARN_MAX_LONGITUDE)
        .bind(STATION_UID_LIKE_PATTERN)
        .fetch_one(pool)
        .await?;

    Ok(LatestMetrics {
        station_count: row.try_get::<i64, _>("station_count")?,
        line_count: row.try_get::<i64, _>("line_count")?,
        operator_count: row.try_get::<i64, _>("operator_count")?,
        blank_station_name_count: row
            .try_get::<Option<i64>, _>("blank_station_name_count")?
            .unwrap_or_default(),
        blank_line_name_count: row
            .try_get::<Option<i64>, _>("blank_line_name_count")?
            .unwrap_or_default(),
        blank_operator_name_count: row
            .try_get::<Option<i64>, _>("blank_operator_name_count")?
            .unwrap_or_default(),
        out_of_range_coordinate_count: row
            .try_get::<Option<i64>, _>("out_of_range_coordinate_count")?
            .unwrap_or_default(),
        suspicious_coordinate_count: row
            .try_get::<Option<i64>, _>("suspicious_coordinate_count")?
            .unwrap_or_default(),
    })
}

async fn fetch_duplicate_station_uid_count(pool: &AnyPool, dialect: SqlDialect) -> Result<i64> {
    let sql = format!(
        "SELECT {} AS duplicate_station_uid_count
         FROM (
           SELECT station_uid
           FROM stations_latest
           WHERE station_uid LIKE ?
           GROUP BY station_uid
           HAVING COUNT(*) > 1
         ) duplicate_station_uids",
        integer_aggregate_sql(dialect, "COUNT(*)"),
    );
    let row = sqlx::query(&dialect.statement(&sql))
        .bind(STATION_UID_LIKE_PATTERN)
        .fetch_one(pool)
        .await?;

    Ok(row.try_get::<i64, _>("duplicate_station_uid_count")?)
}

async fn fetch_change_summary(
    pool: &AnyPool,
    dialect: SqlDialect,
    snapshot_id: i64,
) -> Result<ChangeSummary> {
    let counts_sql = format!(
        "SELECT
           {} AS created_count,
           {} AS updated_count,
           {} AS removed_count
         FROM station_change_events
         WHERE snapshot_id = ?
           AND station_uid LIKE ?",
        integer_aggregate_sql(
            dialect,
            "SUM(CASE WHEN change_kind = 'created' THEN 1 ELSE 0 END)"
        ),
        integer_aggregate_sql(
            dialect,
            "SUM(CASE WHEN change_kind = 'updated' THEN 1 ELSE 0 END)"
        ),
        integer_aggregate_sql(
            dialect,
            "SUM(CASE WHEN change_kind = 'removed' THEN 1 ELSE 0 END)"
        ),
    );
    let counts = sqlx::query(&dialect.statement(&counts_sql))
        .bind(snapshot_id)
        .bind(STATION_UID_LIKE_PATTERN)
        .fetch_one(pool)
        .await?;

    let latest_versions_sql = format!(
        "SELECT {} AS latest_snapshot_version_count
         FROM station_versions
         WHERE snapshot_id = ?
           AND station_uid LIKE ?",
        integer_aggregate_sql(dialect, "COUNT(*)"),
    );
    let latest_versions = sqlx::query(&dialect.statement(&latest_versions_sql))
        .bind(snapshot_id)
        .bind(STATION_UID_LIKE_PATTERN)
        .fetch_one(pool)
        .await?;

    Ok(ChangeSummary {
        created_count: counts
            .try_get::<Option<i64>, _>("created_count")?
            .unwrap_or_default(),
        updated_count: counts
            .try_get::<Option<i64>, _>("updated_count")?
            .unwrap_or_default(),
        removed_count: counts
            .try_get::<Option<i64>, _>("removed_count")?
            .unwrap_or_default(),
        latest_snapshot_version_count: latest_versions
            .try_get::<i64, _>("latest_snapshot_version_count")?,
    })
}

fn threshold_check(name: &'static str, observed: i64, minimum: i64) -> ValidationCheck {
    if observed >= minimum {
        ok_check(name, json!(observed), json!(format!(">={minimum}")), None)
    } else {
        failed_check(name, json!(observed), json!(format!(">={minimum}")), None)
    }
}

fn zero_check(name: &'static str, observed: i64) -> ValidationCheck {
    if observed == 0 {
        ok_check(name, json!(observed), json!(0), None)
    } else {
        failed_check(name, json!(observed), json!(0), None)
    }
}

fn suspicious_coordinate_check(observed: i64) -> ValidationCheck {
    let expected = json!({
        "latitude": format!("{WARN_MIN_LATITUDE}..={WARN_MAX_LATITUDE}"),
        "longitude": format!("{WARN_MIN_LONGITUDE}..={WARN_MAX_LONGITUDE}"),
    });

    if observed == 0 {
        ok_check(
            "suspicious_coordinate_count",
            json!(observed),
            expected,
            None,
        )
    } else {
        warning_check(
            "suspicious_coordinate_count",
            json!(observed),
            expected,
            Some("representative point is inside Japan bounds but outside the tighter expected rail corridor".to_string()),
        )
    }
}

fn change_summary_check(station_count: i64, summary: &ChangeSummary) -> ValidationCheck {
    let expected = json!({
        "latest_snapshot_versions": "created + updated",
        "current_active_stations": ">= latest_snapshot_versions",
    });
    let observed = json!({
        "current_active_stations": station_count,
        "latest_snapshot_versions": summary.latest_snapshot_version_count,
        "created": summary.created_count,
        "updated": summary.updated_count,
        "unchanged_derived": station_count - summary.latest_snapshot_version_count,
        "removed": summary.removed_count,
    });

    let is_consistent = summary.latest_snapshot_version_count
        == summary.created_count + summary.updated_count
        && station_count >= summary.latest_snapshot_version_count;

    if is_consistent {
        ok_check("change_summary_consistent", observed, expected, None)
    } else {
        failed_check(
            "change_summary_consistent",
            observed,
            expected,
            Some(
                "latest snapshot versions should be explainable as created + updated, and active stations must not be fewer than those rows".to_string(),
            ),
        )
    }
}

fn source_url_quality_check(source_url: &str) -> ValidationCheck {
    let trimmed = source_url.trim();

    if trimmed.is_empty() {
        return warning_check(
            "source_url_quality",
            json!("missing"),
            json!("remote_or_canonical_url"),
            Some("source_url is empty".to_string()),
        );
    }

    if trimmed.starts_with("file://") || trimmed.starts_with('/') {
        return warning_check(
            "source_url_quality",
            json!(trimmed),
            json!("remote_or_canonical_url"),
            Some("source_url points to a local file".to_string()),
        );
    }

    ok_check(
        "source_url_quality",
        json!(trimmed),
        json!("remote_or_canonical_url"),
        None,
    )
}

fn ok_check(
    name: &'static str,
    observed: Value,
    expected: Value,
    message: Option<String>,
) -> ValidationCheck {
    build_check(name, ValidationStatus::Ok, observed, expected, message)
}

fn warning_check(
    name: &'static str,
    observed: Value,
    expected: Value,
    message: Option<String>,
) -> ValidationCheck {
    build_check(name, ValidationStatus::Warning, observed, expected, message)
}

fn failed_check(
    name: &'static str,
    observed: Value,
    expected: Value,
    message: Option<String>,
) -> ValidationCheck {
    build_check(name, ValidationStatus::Failed, observed, expected, message)
}

fn build_check(
    name: &'static str,
    status: ValidationStatus,
    observed: Value,
    expected: Value,
    message: Option<String>,
) -> ValidationCheck {
    ValidationCheck {
        name,
        status,
        observed,
        expected,
        message,
    }
}

fn overall_status(checks: &[ValidationCheck], strict: bool) -> ValidationStatus {
    if checks
        .iter()
        .any(|check| matches!(check.status, ValidationStatus::Failed))
    {
        return ValidationStatus::Failed;
    }

    if checks
        .iter()
        .any(|check| matches!(check.status, ValidationStatus::Warning))
    {
        if strict {
            ValidationStatus::Failed
        } else {
            ValidationStatus::Warning
        }
    } else {
        ValidationStatus::Ok
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

fn decode_required_string(row: &AnyRow, column: &str) -> Result<String> {
    match row.try_get::<String, _>(column) {
        Ok(value) => Ok(value),
        Err(_) => Ok(String::from_utf8(decode_bytes(row, column)?)?),
    }
}

fn decode_optional_string(row: &AnyRow, column: &str) -> Result<Option<String>> {
    match row.try_get::<Option<String>, _>(column) {
        Ok(value) => Ok(value),
        Err(_) => row
            .try_get::<Option<Vec<u8>>, _>(column)
            .map_err(anyhow::Error::from)?
            .map(String::from_utf8)
            .transpose()
            .map_err(Into::into),
    }
}

fn decode_bytes(row: &AnyRow, column: &str) -> Result<Vec<u8>, sqlx::Error> {
    row.try_get::<Vec<u8>, _>(column)
}

fn integer_aggregate_sql(dialect: SqlDialect, expr: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("CAST(COALESCE({expr}, 0) AS SIGNED)"),
        SqlDialect::Postgres | SqlDialect::Sqlite => {
            format!("CAST(COALESCE({expr}, 0) AS BIGINT)")
        }
    }
}

fn distinct_text_count_sql(dialect: SqlDialect, column: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("COUNT(DISTINCT CAST({column} AS BINARY))"),
        SqlDialect::Postgres | SqlDialect::Sqlite => format!("COUNT(DISTINCT {column})"),
    }
}

#[cfg(test)]
mod tests {
    use sqlx::{any::AnyPoolOptions, AnyPool};
    use station_crawler::n02::ingest_snapshot;
    use station_shared::db::{ensure_sqlx_drivers, SqlDialect};

    use super::*;

    #[tokio::test]
    async fn validate_ingest_happy_path_on_sqlite() {
        let pool = test_pool().await;
        let zip_bytes = snapshot_zip_bytes(
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
            }"#,
        );

        ingest_snapshot(
            &pool,
            SqlDialect::Sqlite,
            "https://example.com/N02-24_GML.zip",
            "/tmp/N02-24_GML.zip",
            &zip_bytes,
        )
        .await
        .unwrap();

        let report = validate_ingest(
            &pool,
            SqlDialect::Sqlite,
            &ValidateIngestArgs {
                min_stations: 2,
                min_lines: 2,
                min_operators: 2,
                ..ValidateIngestArgs::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(report.status, ValidationStatus::Ok);
        assert_eq!(report.snapshot_id, Some(1));
        assert_eq!(
            report
                .checks
                .iter()
                .find(|check| check.name == "change_summary_consistent")
                .unwrap()
                .status,
            ValidationStatus::Ok
        );
    }

    #[tokio::test]
    async fn validate_ingest_fails_on_low_count_fixture() {
        let pool = test_pool().await;
        let zip_bytes = snapshot_zip_bytes(
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
                }
              ]
            }"#,
        );

        ingest_snapshot(
            &pool,
            SqlDialect::Sqlite,
            "https://example.com/N02-24_GML.zip",
            "/tmp/N02-24_GML.zip",
            &zip_bytes,
        )
        .await
        .unwrap();

        let report = validate_ingest(
            &pool,
            SqlDialect::Sqlite,
            &ValidateIngestArgs {
                min_stations: 2,
                min_lines: 1,
                min_operators: 1,
                ..ValidateIngestArgs::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(report.status, ValidationStatus::Failed);
        assert_eq!(
            report
                .checks
                .iter()
                .find(|check| check.name == "min_station_count")
                .unwrap()
                .status,
            ValidationStatus::Failed
        );
    }

    #[tokio::test]
    async fn validate_ingest_fails_on_blank_names() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 1, "https://example.com/N02-24_GML.zip").await;
        insert_identity(&pool, "stn_n02_blank", "空").await;
        insert_station_version(
            &pool,
            1,
            StationVersionSeed::new("stn_n02_blank", "", "京王線", "京王電鉄", 35.69, 139.70),
        )
        .await;
        insert_change_event(&pool, 1, "stn_n02_blank", "created").await;

        let report = validate_ingest(
            &pool,
            SqlDialect::Sqlite,
            &ValidateIngestArgs {
                min_stations: 1,
                min_lines: 1,
                min_operators: 1,
                ..ValidateIngestArgs::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(report.status, ValidationStatus::Failed);
        assert_eq!(
            report
                .checks
                .iter()
                .find(|check| check.name == "blank_station_name_count")
                .unwrap()
                .status,
            ValidationStatus::Failed
        );
    }

    #[tokio::test]
    async fn validate_ingest_fails_on_duplicate_latest_station_uid() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 1, "https://example.com/N02-24_GML.zip").await;
        insert_snapshot(&pool, 2, "https://example.com/N02-25_GML.zip").await;
        insert_identity(&pool, "stn_n02_duplicate", "新宿").await;
        insert_station_version(
            &pool,
            1,
            StationVersionSeed::new(
                "stn_n02_duplicate",
                "新宿",
                "京王線",
                "京王電鉄",
                35.69,
                139.70,
            ),
        )
        .await;
        insert_station_version(
            &pool,
            2,
            StationVersionSeed::new(
                "stn_n02_duplicate",
                "新宿",
                "京王線",
                "京王電鉄",
                35.69,
                139.70,
            ),
        )
        .await;
        insert_change_event(&pool, 2, "stn_n02_duplicate", "created").await;

        let report = validate_ingest(
            &pool,
            SqlDialect::Sqlite,
            &ValidateIngestArgs {
                min_stations: 2,
                min_lines: 1,
                min_operators: 1,
                ..ValidateIngestArgs::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(report.status, ValidationStatus::Failed);
        assert_eq!(
            report
                .checks
                .iter()
                .find(|check| check.name == "duplicate_latest_station_uid_count")
                .unwrap()
                .status,
            ValidationStatus::Failed
        );
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

    async fn insert_snapshot(pool: &AnyPool, id: i64, source_url: &str) {
        sqlx::query(
            "INSERT INTO source_snapshots (id, source_name, source_kind, source_version, source_url, source_sha256)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(SOURCE_NAME)
        .bind("geojson_zip_entry")
        .bind("N02-24")
        .bind(source_url)
        .bind(format!("sha-{id}"))
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_identity(pool: &AnyPool, station_uid: &str, canonical_name: &str) {
        sqlx::query(
            "INSERT INTO station_identities (station_uid, canonical_name)
             VALUES (?, ?)",
        )
        .bind(station_uid)
        .bind(canonical_name)
        .execute(pool)
        .await
        .unwrap();
    }

    struct StationVersionSeed<'a> {
        station_uid: &'a str,
        station_name: &'a str,
        line_name: &'a str,
        operator_name: &'a str,
        latitude: f64,
        longitude: f64,
    }

    impl<'a> StationVersionSeed<'a> {
        fn new(
            station_uid: &'a str,
            station_name: &'a str,
            line_name: &'a str,
            operator_name: &'a str,
            latitude: f64,
            longitude: f64,
        ) -> Self {
            Self {
                station_uid,
                station_name,
                line_name,
                operator_name,
                latitude,
                longitude,
            }
        }
    }

    async fn insert_station_version(
        pool: &AnyPool,
        snapshot_id: i64,
        station: StationVersionSeed<'_>,
    ) {
        sqlx::query(
            "INSERT INTO station_versions (
               station_uid,
               snapshot_id,
               station_name,
               line_name,
               operator_name,
               latitude,
               longitude,
               status,
               change_hash
             ) VALUES (?, ?, ?, ?, ?, ?, ?, 'active', ?)",
        )
        .bind(station.station_uid)
        .bind(snapshot_id)
        .bind(station.station_name)
        .bind(station.line_name)
        .bind(station.operator_name)
        .bind(station.latitude)
        .bind(station.longitude)
        .bind(format!(
            "{}:{snapshot_id}:{}",
            station.station_uid, station.station_name
        ))
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_change_event(
        pool: &AnyPool,
        snapshot_id: i64,
        station_uid: &str,
        change_kind: &str,
    ) {
        sqlx::query(
            "INSERT INTO station_change_events (snapshot_id, station_uid, change_kind)
             VALUES (?, ?, ?)",
        )
        .bind(snapshot_id)
        .bind(station_uid)
        .bind(change_kind)
        .execute(pool)
        .await
        .unwrap();
    }

    fn snapshot_zip_bytes(geojson: &str) -> Vec<u8> {
        use std::io::{Cursor, Write};

        use zip::{write::SimpleFileOptions, ZipWriter};

        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("UTF-8/N02-24_Station.geojson", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(geojson.as_bytes()).unwrap();
        writer.finish().unwrap().into_inner()
    }
}
