use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{any::AnyRow, Any, AnyPool, Row, Transaction};
use station_shared::db::SqlDialect;
use zip::ZipArchive;

const SOURCE_NAME: &str = "ksj_n02_station";
const SOURCE_KIND: &str = "geojson_zip_entry";
const STATION_UID_PREFIX: &str = "stn_n02_";
const STATION_GEOJSON_PATH: &str = "UTF-8/N02-24_Station.geojson";
const DEFAULT_INGEST_WRITE_CHUNK_SIZE: usize = 1_000;
const DEFAULT_INGEST_CLOSE_CHUNK_SIZE: usize = 1_000;
const SQLITE_MAX_VARIABLE_NUMBER: usize = 999;
const SQLITE_SAFE_WRITE_CHUNK_SIZE: usize = SQLITE_MAX_VARIABLE_NUMBER / 13;
const SQLITE_SAFE_CLOSE_CHUNK_SIZE: usize = SQLITE_MAX_VARIABLE_NUMBER - 1;

#[derive(Clone, Copy, Debug)]
pub struct PersistChunkConfig {
    pub write_chunk_size: usize,
    pub close_chunk_size: usize,
}

impl Default for PersistChunkConfig {
    fn default() -> Self {
        Self {
            write_chunk_size: DEFAULT_INGEST_WRITE_CHUNK_SIZE,
            close_chunk_size: DEFAULT_INGEST_CLOSE_CHUNK_SIZE,
        }
    }
}

impl PersistChunkConfig {
    pub fn for_dialect(dialect: SqlDialect) -> Self {
        let write_chunk_size = match dialect {
            SqlDialect::Mysql => 200,
            SqlDialect::Postgres => DEFAULT_INGEST_WRITE_CHUNK_SIZE,
            SqlDialect::Sqlite => SQLITE_SAFE_WRITE_CHUNK_SIZE,
        };
        let close_chunk_size = match dialect {
            SqlDialect::Postgres | SqlDialect::Mysql => DEFAULT_INGEST_CLOSE_CHUNK_SIZE,
            SqlDialect::Sqlite => SQLITE_SAFE_CLOSE_CHUNK_SIZE,
        };

        Self {
            write_chunk_size,
            close_chunk_size,
        }
    }

    pub fn clamp_for_dialect(self, dialect: SqlDialect) -> Self {
        match dialect {
            SqlDialect::Sqlite => Self {
                write_chunk_size: self.write_chunk_size.min(SQLITE_SAFE_WRITE_CHUNK_SIZE),
                close_chunk_size: self.close_chunk_size.min(SQLITE_SAFE_CLOSE_CHUNK_SIZE),
            },
            SqlDialect::Postgres | SqlDialect::Mysql => self,
        }
    }

    fn validate(self) -> Result<Self> {
        if self.write_chunk_size == 0 {
            return Err(anyhow!(
                "invalid PersistChunkConfig: write_chunk_size must be greater than 0"
            ));
        }
        if self.close_chunk_size == 0 {
            return Err(anyhow!(
                "invalid PersistChunkConfig: close_chunk_size must be greater than 0"
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Serialize)]
pub struct IngestReport {
    pub source_name: &'static str,
    pub source_version: Option<String>,
    pub source_url: String,
    pub source_sha256: String,
    pub saved_to: String,
    pub snapshot_id: Option<i64>,
    pub parsed_features: usize,
    pub parsed_stations: usize,
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub skipped_existing_snapshot: bool,
    pub load_ms: u64,
    pub save_zip_ms: u64,
    pub extract_ms: u64,
    pub parse_ms: u64,
    pub diff_ms: u64,
    pub persist_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug)]
struct ParsedSnapshot {
    source_version: Option<String>,
    parsed_features: usize,
    stations: Vec<ParsedStation>,
}

#[derive(Debug)]
struct TimedParsedSnapshot {
    snapshot: ParsedSnapshot,
    extract_ms: u64,
    parse_ms: u64,
}

#[derive(Clone, Debug)]
struct ParsedStation {
    station_uid: String,
    source_station_code: Option<String>,
    source_group_code: Option<String>,
    station_name: String,
    line_name: String,
    operator_name: String,
    latitude: f64,
    longitude: f64,
    geometry_geojson: String,
    status: &'static str,
    change_hash: String,
}

#[derive(Debug, Default)]
struct DiffPlan {
    identity_name_mutations: Vec<IdentityNameMutation>,
    created_stations: Vec<ParsedStation>,
    updated_stations: Vec<UpdatedStationChange>,
    removed_stations: Vec<ExistingVersion>,
    unchanged: usize,
}

#[derive(Clone, Debug)]
struct UpdatedStationChange {
    before: ExistingVersion,
    after: ParsedStation,
}

#[derive(Clone, Debug)]
struct IdentityNameMutation {
    station_uid: String,
    canonical_name: String,
    kind: IdentityNameMutationKind,
}

#[derive(Debug)]
struct ChangeEventInsert {
    station_uid: String,
    change_kind: &'static str,
    before_version_id: Option<i64>,
    after_version_id: Option<i64>,
    detail_json: String,
}

#[derive(Clone, Copy, Debug)]
enum IdentityNameMutationKind {
    Insert,
    Update,
}

#[derive(Clone, Debug)]
struct ExistingVersion {
    id: i64,
    station_uid: String,
    source_station_code: Option<String>,
    source_group_code: Option<String>,
    station_name: String,
    line_name: String,
    operator_name: String,
    latitude: f64,
    longitude: f64,
    geometry_geojson: Option<String>,
    status: String,
    change_hash: String,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct LogicalStationKey {
    source_station_code: String,
    source_group_code: String,
    station_name: String,
    line_name: String,
    operator_name: String,
}

#[derive(Debug, Deserialize)]
struct RawFeatureCollection {
    features: Vec<RawFeature>,
}

#[derive(Debug, Deserialize)]
struct RawFeature {
    properties: RawProperties,
    geometry: RawGeometry,
}

#[derive(Debug, Deserialize)]
struct RawProperties {
    #[serde(rename = "N02_003")]
    line_name: String,
    #[serde(rename = "N02_004")]
    operator_name: String,
    #[serde(rename = "N02_005")]
    station_name: String,
    #[serde(rename = "N02_005c")]
    source_station_code: String,
    #[serde(rename = "N02_005g")]
    source_group_code: String,
}

#[derive(Debug, Deserialize)]
struct RawGeometry {
    #[serde(rename = "type")]
    geometry_type: String,
    coordinates: Value,
}

pub async fn ingest_snapshot(
    pool: &AnyPool,
    dialect: SqlDialect,
    source_url: &str,
    saved_to: &str,
    zip_bytes: &[u8],
) -> Result<IngestReport> {
    ingest_snapshot_with_config(
        pool,
        dialect,
        source_url,
        saved_to,
        zip_bytes,
        PersistChunkConfig::for_dialect(dialect),
    )
    .await
}

pub async fn ingest_snapshot_with_config(
    pool: &AnyPool,
    dialect: SqlDialect,
    source_url: &str,
    saved_to: &str,
    zip_bytes: &[u8],
    chunk_config: PersistChunkConfig,
) -> Result<IngestReport> {
    let chunk_config = chunk_config.clamp_for_dialect(dialect).validate()?;
    let total_start = Instant::now();
    let source_sha256 = sha256_hex(zip_bytes);
    let parsed_snapshot = parse_snapshot_timed(zip_bytes, source_url)?;

    let mut report = persist_snapshot(
        pool,
        dialect,
        source_url,
        saved_to,
        &source_sha256,
        &parsed_snapshot.snapshot,
        chunk_config,
    )
    .await?;
    report.source_sha256 = source_sha256;
    report.extract_ms = parsed_snapshot.extract_ms;
    report.parse_ms = parsed_snapshot.parse_ms;
    report.total_ms = duration_ms(total_start.elapsed());
    Ok(report)
}

fn parse_snapshot_timed(zip_bytes: &[u8], source_url: &str) -> Result<TimedParsedSnapshot> {
    let extract_start = Instant::now();
    let (entry_name, geojson_bytes) = extract_station_geojson(zip_bytes)?;
    let extract_ms = duration_ms(extract_start.elapsed());

    let parse_start = Instant::now();
    let source_version = detect_source_version(source_url, &entry_name);
    let snapshot = parse_feature_collection(&geojson_bytes, source_version)?;
    let parse_ms = duration_ms(parse_start.elapsed());

    Ok(TimedParsedSnapshot {
        snapshot,
        extract_ms,
        parse_ms,
    })
}

fn parse_feature_collection(
    geojson_bytes: &[u8],
    source_version: Option<String>,
) -> Result<ParsedSnapshot> {
    let feature_collection: RawFeatureCollection =
        serde_json::from_slice(geojson_bytes).context("failed to parse N02 station GeoJSON")?;

    let parsed_features = feature_collection.features.len();
    let mut grouped: BTreeMap<LogicalStationKey, Vec<Vec<[f64; 2]>>> = BTreeMap::new();

    for (index, feature) in feature_collection.features.into_iter().enumerate() {
        let key = LogicalStationKey::from(feature.properties);
        let segments = geometry_to_segments(feature.geometry)
            .with_context(|| format!("invalid geometry at feature index {index}"))?;

        grouped.entry(key).or_default().extend(segments);
    }

    let stations = grouped
        .into_iter()
        .map(|(key, segments)| ParsedStation::from_segments(key, segments))
        .collect::<Result<Vec<_>>>()?;

    Ok(ParsedSnapshot {
        source_version,
        parsed_features,
        stations,
    })
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

async fn persist_snapshot(
    pool: &AnyPool,
    dialect: SqlDialect,
    source_url: &str,
    saved_to: &str,
    source_sha256: &str,
    snapshot: &ParsedSnapshot,
    chunk_config: PersistChunkConfig,
) -> Result<IngestReport> {
    let mut tx = pool.begin().await?;

    let preflight_start = Instant::now();
    if let Some(snapshot_id) = fetch_snapshot_id(&mut tx, dialect, source_sha256).await? {
        return Ok(IngestReport {
            source_name: SOURCE_NAME,
            source_version: snapshot.source_version.clone(),
            source_url: source_url.to_string(),
            source_sha256: source_sha256.to_string(),
            saved_to: saved_to.to_string(),
            snapshot_id: Some(snapshot_id),
            parsed_features: snapshot.parsed_features,
            parsed_stations: snapshot.stations.len(),
            created: 0,
            updated: 0,
            unchanged: 0,
            removed: 0,
            skipped_existing_snapshot: true,
            load_ms: 0,
            save_zip_ms: 0,
            extract_ms: 0,
            parse_ms: 0,
            diff_ms: 0,
            persist_ms: duration_ms(preflight_start.elapsed()),
            total_ms: 0,
        });
    }

    insert_source_snapshot(
        &mut tx,
        dialect,
        source_url,
        source_sha256,
        snapshot.source_version.as_deref(),
    )
    .await?;
    let snapshot_id = fetch_snapshot_id(&mut tx, dialect, source_sha256)
        .await?
        .context("failed to load inserted snapshot id")?;
    let preflight_duration = preflight_start.elapsed();

    let now = Utc::now().to_rfc3339();
    let diff_start = Instant::now();
    let identity_names = fetch_identity_names(&mut tx, dialect).await?;
    let latest_versions = fetch_latest_versions(&mut tx, dialect).await?;
    let diff_plan = build_diff_plan(snapshot, identity_names, latest_versions);
    let diff_ms = duration_ms(diff_start.elapsed());

    let persist_start = Instant::now();
    apply_identity_name_mutations(
        &mut tx,
        dialect,
        &diff_plan.identity_name_mutations,
        chunk_config.write_chunk_size,
    )
    .await?;

    let stale_version_ids = diff_plan
        .updated_stations
        .iter()
        .map(|change| change.before.id)
        .chain(diff_plan.removed_stations.iter().map(|stale| stale.id))
        .collect::<Vec<_>>();
    close_station_versions(
        &mut tx,
        dialect,
        &stale_version_ids,
        &now,
        chunk_config.close_chunk_size,
    )
    .await?;

    let created = diff_plan.created_stations.len();
    let updated = diff_plan.updated_stations.len();
    let removed = diff_plan.removed_stations.len();

    let version_inserts = diff_plan
        .created_stations
        .iter()
        .cloned()
        .chain(
            diff_plan
                .updated_stations
                .iter()
                .map(|change| change.after.clone()),
        )
        .collect::<Vec<_>>();

    insert_station_versions(
        &mut tx,
        dialect,
        snapshot_id,
        &version_inserts,
        &now,
        chunk_config.write_chunk_size,
    )
    .await?;
    let inserted_version_ids = fetch_inserted_version_ids(&mut tx, dialect, snapshot_id).await?;
    let change_events = build_change_events(snapshot_id, &diff_plan, &inserted_version_ids)?;
    insert_change_events(
        &mut tx,
        dialect,
        snapshot_id,
        &change_events,
        chunk_config.write_chunk_size,
    )
    .await?;

    tx.commit().await?;
    let persist_duration = preflight_duration + persist_start.elapsed();

    Ok(IngestReport {
        source_name: SOURCE_NAME,
        source_version: snapshot.source_version.clone(),
        source_url: source_url.to_string(),
        source_sha256: source_sha256.to_string(),
        saved_to: saved_to.to_string(),
        snapshot_id: Some(snapshot_id),
        parsed_features: snapshot.parsed_features,
        parsed_stations: snapshot.stations.len(),
        created,
        updated,
        unchanged: diff_plan.unchanged,
        removed,
        skipped_existing_snapshot: false,
        load_ms: 0,
        save_zip_ms: 0,
        extract_ms: 0,
        parse_ms: 0,
        diff_ms,
        persist_ms: duration_ms(persist_duration),
        total_ms: 0,
    })
}

async fn fetch_snapshot_id(
    tx: &mut Transaction<'_, Any>,
    dialect: SqlDialect,
    source_sha256: &str,
) -> Result<Option<i64>> {
    let row = sqlx::query(&dialect.statement(
        "SELECT id
         FROM source_snapshots
         WHERE source_name = ? AND source_sha256 = ?
        LIMIT 1",
    ))
    .bind(SOURCE_NAME)
    .bind(source_sha256)
    .fetch_optional(&mut **tx)
    .await?;

    row.map(|row| row.try_get::<i64, _>("id"))
        .transpose()
        .map_err(Into::into)
}

async fn insert_source_snapshot(
    tx: &mut Transaction<'_, Any>,
    dialect: SqlDialect,
    source_url: &str,
    source_sha256: &str,
    source_version: Option<&str>,
) -> Result<()> {
    sqlx::query(&dialect.statement(
        "INSERT INTO source_snapshots (
           source_name,
           source_kind,
           source_version,
           source_url,
           source_sha256
         ) VALUES (?, ?, ?, ?, ?)",
    ))
    .bind(SOURCE_NAME)
    .bind(SOURCE_KIND)
    .bind(source_version)
    .bind(source_url)
    .bind(source_sha256)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn fetch_identity_names(
    tx: &mut Transaction<'_, Any>,
    dialect: SqlDialect,
) -> Result<BTreeMap<String, String>> {
    let rows = sqlx::query(&dialect.statement(
        "SELECT station_uid, canonical_name
         FROM station_identities
         WHERE substr(station_uid, 1, 8) = ?",
    ))
    .bind(STATION_UID_PREFIX)
    .fetch_all(&mut **tx)
    .await?;

    let mut names = BTreeMap::new();
    for row in rows {
        names.insert(
            row_string(&row, "station_uid")?,
            row_string(&row, "canonical_name")?,
        );
    }

    Ok(names)
}

async fn apply_identity_name_mutations(
    tx: &mut Transaction<'_, Any>,
    dialect: SqlDialect,
    mutations: &[IdentityNameMutation],
    chunk_size: usize,
) -> Result<()> {
    let inserts = mutations
        .iter()
        .filter(|mutation| matches!(mutation.kind, IdentityNameMutationKind::Insert))
        .collect::<Vec<_>>();
    for chunk in inserts.chunks(chunk_size) {
        let values = std::iter::repeat_n("(?, ?)", chunk.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql =
            format!("INSERT INTO station_identities (station_uid, canonical_name) VALUES {values}");
        let statement = dialect.statement(&sql);
        let mut query = sqlx::query(&statement);
        for mutation in chunk {
            query = query
                .bind(&mutation.station_uid)
                .bind(&mutation.canonical_name);
        }
        query.execute(&mut **tx).await?;
    }

    let updates = mutations
        .iter()
        .filter(|mutation| matches!(mutation.kind, IdentityNameMutationKind::Update))
        .collect::<Vec<_>>();
    for chunk in updates.chunks(chunk_size) {
        let mut sql = String::from(
            "UPDATE station_identities
             SET canonical_name = CASE station_uid",
        );
        for _ in chunk {
            sql.push_str(" WHEN ? THEN ?");
        }
        let where_placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(", ");
        sql.push_str(" ELSE canonical_name END WHERE station_uid IN (");
        sql.push_str(&where_placeholders);
        sql.push(')');

        let statement = dialect.statement(&sql);
        let mut query = sqlx::query(&statement);
        for mutation in chunk {
            query = query
                .bind(&mutation.station_uid)
                .bind(&mutation.canonical_name);
        }
        for mutation in chunk {
            query = query.bind(&mutation.station_uid);
        }
        query.execute(&mut **tx).await?;
    }

    Ok(())
}

fn queue_identity_name_mutation(
    identities: &mut BTreeMap<String, String>,
    identity_name_mutations: &mut Vec<IdentityNameMutation>,
    station: &ParsedStation,
) {
    match identities.get(&station.station_uid) {
        None => {
            identity_name_mutations.push(IdentityNameMutation {
                station_uid: station.station_uid.clone(),
                canonical_name: station.station_name.clone(),
                kind: IdentityNameMutationKind::Insert,
            });
        }
        Some(existing_name) if existing_name != &station.station_name => {
            identity_name_mutations.push(IdentityNameMutation {
                station_uid: station.station_uid.clone(),
                canonical_name: station.station_name.clone(),
                kind: IdentityNameMutationKind::Update,
            });
        }
        Some(_) => {}
    }

    identities.insert(station.station_uid.clone(), station.station_name.clone());
}

fn build_diff_plan(
    snapshot: &ParsedSnapshot,
    mut identity_names: BTreeMap<String, String>,
    mut latest_versions: BTreeMap<String, ExistingVersion>,
) -> DiffPlan {
    let mut plan = DiffPlan::default();

    for station in &snapshot.stations {
        queue_identity_name_mutation(
            &mut identity_names,
            &mut plan.identity_name_mutations,
            station,
        );

        match latest_versions.remove(&station.station_uid) {
            None => plan.created_stations.push(station.clone()),
            Some(existing) if existing.change_hash == station.change_hash => {
                plan.unchanged += 1;
            }
            Some(existing) => plan.updated_stations.push(UpdatedStationChange {
                before: existing,
                after: station.clone(),
            }),
        }
    }

    plan.removed_stations = latest_versions.into_values().collect();
    plan
}

async fn fetch_latest_versions(
    tx: &mut Transaction<'_, Any>,
    dialect: SqlDialect,
) -> Result<BTreeMap<String, ExistingVersion>> {
    let rows = sqlx::query(&dialect.statement(
        "SELECT
           id,
           station_uid,
           source_station_code,
           source_group_code,
           station_name,
           line_name,
           operator_name,
           latitude,
           longitude,
           geometry_geojson,
           status,
           change_hash
         FROM station_versions
         WHERE valid_to IS NULL
           AND substr(station_uid, 1, 8) = ?",
    ))
    .bind(STATION_UID_PREFIX)
    .fetch_all(&mut **tx)
    .await?;

    let mut versions = BTreeMap::new();
    for row in rows {
        let version = ExistingVersion {
            id: row.try_get::<i64, _>("id")?,
            station_uid: row_string(&row, "station_uid")?,
            source_station_code: row_optional_string(&row, "source_station_code")?,
            source_group_code: row_optional_string(&row, "source_group_code")?,
            station_name: row_string(&row, "station_name")?,
            line_name: row_string(&row, "line_name")?,
            operator_name: row_string(&row, "operator_name")?,
            latitude: row.try_get::<f64, _>("latitude")?,
            longitude: row.try_get::<f64, _>("longitude")?,
            geometry_geojson: row_optional_string(&row, "geometry_geojson")?,
            status: row_string(&row, "status")?,
            change_hash: row_string(&row, "change_hash")?,
        };

        versions.insert(version.station_uid.clone(), version);
    }

    Ok(versions)
}

async fn close_station_versions(
    tx: &mut Transaction<'_, Any>,
    dialect: SqlDialect,
    version_ids: &[i64],
    valid_to: &str,
    chunk_size: usize,
) -> Result<()> {
    for chunk in version_ids.chunks(chunk_size) {
        let placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "UPDATE station_versions
             SET valid_to = {}
             WHERE id IN ({placeholders}) AND valid_to IS NULL",
            dialect.timestamp_parameter()
        );

        let statement = dialect.statement(&sql);
        let mut query = sqlx::query(&statement).bind(valid_to);
        for version_id in chunk {
            query = query.bind(*version_id);
        }
        query.execute(&mut **tx).await?;
    }

    Ok(())
}

async fn insert_station_versions(
    tx: &mut Transaction<'_, Any>,
    dialect: SqlDialect,
    snapshot_id: i64,
    stations: &[ParsedStation],
    valid_from: &str,
    chunk_size: usize,
) -> Result<()> {
    for chunk in stations.chunks(chunk_size) {
        let row_sql = format!(
            "(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, {}, ?)",
            dialect.timestamp_parameter()
        );
        let values = std::iter::repeat_n(row_sql.as_str(), chunk.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "INSERT INTO station_versions (
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
               valid_from,
               change_hash
             ) VALUES {values}"
        );

        let statement = dialect.statement(&sql);
        let mut query = sqlx::query(&statement);
        for station in chunk {
            query = query
                .bind(&station.station_uid)
                .bind(snapshot_id)
                .bind(station.source_station_code.as_deref())
                .bind(station.source_group_code.as_deref())
                .bind(&station.station_name)
                .bind(&station.line_name)
                .bind(&station.operator_name)
                .bind(station.latitude)
                .bind(station.longitude)
                .bind(&station.geometry_geojson)
                .bind(station.status)
                .bind(valid_from)
                .bind(&station.change_hash);
        }
        query.execute(&mut **tx).await?;
    }

    Ok(())
}

async fn fetch_inserted_version_ids(
    tx: &mut Transaction<'_, Any>,
    dialect: SqlDialect,
    snapshot_id: i64,
) -> Result<BTreeMap<String, i64>> {
    let rows = sqlx::query(&dialect.statement(
        "SELECT station_uid, id
         FROM station_versions
         WHERE snapshot_id = ?",
    ))
    .bind(snapshot_id)
    .fetch_all(&mut **tx)
    .await?;

    let mut version_ids = BTreeMap::new();
    for row in rows {
        version_ids.insert(
            row_string(&row, "station_uid")?,
            row.try_get::<i64, _>("id")?,
        );
    }

    Ok(version_ids)
}

fn build_change_events(
    snapshot_id: i64,
    diff_plan: &DiffPlan,
    inserted_version_ids: &BTreeMap<String, i64>,
) -> Result<Vec<ChangeEventInsert>> {
    let mut change_events = Vec::with_capacity(
        diff_plan.created_stations.len()
            + diff_plan.updated_stations.len()
            + diff_plan.removed_stations.len(),
    );

    for station in &diff_plan.created_stations {
        let after_version_id = inserted_version_ids
            .get(&station.station_uid)
            .copied()
            .with_context(|| {
                format!(
                    "missing inserted version id for created station {} in snapshot {snapshot_id}",
                    station.station_uid
                )
            })?;
        change_events.push(ChangeEventInsert {
            station_uid: station.station_uid.clone(),
            change_kind: "created",
            before_version_id: None,
            after_version_id: Some(after_version_id),
            detail_json: created_detail_json(station).to_string(),
        });
    }

    for change in &diff_plan.updated_stations {
        let after_version_id = inserted_version_ids
            .get(&change.after.station_uid)
            .copied()
            .with_context(|| {
                format!(
                    "missing inserted version id for updated station {} in snapshot {snapshot_id}",
                    change.after.station_uid
                )
            })?;
        change_events.push(ChangeEventInsert {
            station_uid: change.after.station_uid.clone(),
            change_kind: "updated",
            before_version_id: Some(change.before.id),
            after_version_id: Some(after_version_id),
            detail_json: updated_detail_json(&change.before, &change.after).to_string(),
        });
    }

    for stale in &diff_plan.removed_stations {
        change_events.push(ChangeEventInsert {
            station_uid: stale.station_uid.clone(),
            change_kind: "removed",
            before_version_id: Some(stale.id),
            after_version_id: None,
            detail_json: removed_detail_json(stale).to_string(),
        });
    }

    Ok(change_events)
}

async fn insert_change_events(
    tx: &mut Transaction<'_, Any>,
    dialect: SqlDialect,
    snapshot_id: i64,
    change_events: &[ChangeEventInsert],
    chunk_size: usize,
) -> Result<()> {
    for chunk in change_events.chunks(chunk_size) {
        let values = std::iter::repeat_n("(?, ?, ?, ?, ?, ?)", chunk.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "INSERT INTO station_change_events (
               snapshot_id,
               station_uid,
               change_kind,
               before_version_id,
               after_version_id,
               detail_json
             ) VALUES {values}"
        );

        let statement = dialect.statement(&sql);
        let mut query = sqlx::query(&statement);
        for event in chunk {
            query = query
                .bind(snapshot_id)
                .bind(&event.station_uid)
                .bind(event.change_kind)
                .bind(event.before_version_id)
                .bind(event.after_version_id)
                .bind(&event.detail_json);
        }
        query.execute(&mut **tx).await?;
    }

    Ok(())
}

impl From<RawProperties> for LogicalStationKey {
    fn from(properties: RawProperties) -> Self {
        Self {
            source_station_code: normalized_text(properties.source_station_code),
            source_group_code: normalized_text(properties.source_group_code),
            station_name: normalized_text(properties.station_name),
            line_name: normalized_text(properties.line_name),
            operator_name: normalized_text(properties.operator_name),
        }
    }
}

impl ParsedStation {
    fn from_segments(key: LogicalStationKey, segments: Vec<Vec<[f64; 2]>>) -> Result<Self> {
        let (longitude, latitude) =
            representative_point(&segments).context("failed to calculate representative point")?;
        let geometry_geojson = geometry_geojson_string(&segments)?;
        let source_station_code = optional_text(&key.source_station_code);
        let source_group_code = optional_text(&key.source_group_code);

        let mut station = Self {
            station_uid: build_station_uid(&key),
            source_station_code,
            source_group_code,
            station_name: key.station_name,
            line_name: key.line_name,
            operator_name: key.operator_name,
            latitude,
            longitude,
            geometry_geojson,
            status: "active",
            change_hash: String::new(),
        };

        station.change_hash = station.build_change_hash();
        Ok(station)
    }

    fn build_change_hash(&self) -> String {
        let material = format!(
            "{}|{}|{}|{}|{}|{}|{:.8}|{:.8}|{}|{}",
            self.source_station_code.as_deref().unwrap_or_default(),
            self.source_group_code.as_deref().unwrap_or_default(),
            self.station_name,
            self.line_name,
            self.operator_name,
            self.station_uid,
            self.latitude,
            self.longitude,
            self.status,
            self.geometry_geojson
        );

        sha256_hex(material.as_bytes())
    }
}

fn extract_station_geojson(zip_bytes: &[u8]) -> Result<(String, Vec<u8>)> {
    let mut archive = ZipArchive::new(Cursor::new(zip_bytes)).context("invalid snapshot ZIP")?;
    let entry_name = find_station_geojson_entry_name(&mut archive)?;
    let mut file = archive
        .by_name(&entry_name)
        .with_context(|| format!("missing station GeoJSON entry: {entry_name}"))?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    Ok((entry_name, bytes))
}

fn find_station_geojson_entry_name<R>(archive: &mut ZipArchive<R>) -> Result<String>
where
    R: Read + std::io::Seek,
{
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        let name = file.name().to_string();
        if name == STATION_GEOJSON_PATH {
            return Ok(name);
        }
    }

    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        let name = file.name().to_string();
        if name.contains("UTF-8/") && name.ends_with("_Station.geojson") {
            return Ok(name);
        }
    }

    Err(anyhow!("station GeoJSON entry not found in snapshot ZIP"))
}

fn detect_source_version(source_url: &str, entry_name: &str) -> Option<String> {
    source_url
        .rsplit('/')
        .next()
        .and_then(|name| name.strip_suffix("_GML.zip"))
        .map(str::to_string)
        .or_else(|| {
            entry_name
                .rsplit('/')
                .next()
                .and_then(|name| name.strip_suffix("_Station.geojson"))
                .map(str::to_string)
        })
}

fn geometry_to_segments(geometry: RawGeometry) -> Result<Vec<Vec<[f64; 2]>>> {
    match geometry.geometry_type.as_str() {
        "LineString" => Ok(vec![value_to_line_string(&geometry.coordinates)?]),
        "MultiLineString" => {
            let values = geometry
                .coordinates
                .as_array()
                .context("MultiLineString coordinates must be an array")?;

            values
                .iter()
                .map(value_to_line_string)
                .collect::<Result<Vec<_>>>()
        }
        other => Err(anyhow!("unsupported geometry type: {other}")),
    }
}

fn value_to_line_string(value: &Value) -> Result<Vec<[f64; 2]>> {
    let coordinates = value
        .as_array()
        .context("LineString coordinates must be an array")?;

    let points = coordinates
        .iter()
        .map(|coordinate| {
            let pair = coordinate
                .as_array()
                .context("coordinate must be an array with longitude and latitude")?;

            if pair.len() < 2 {
                return Err(anyhow!("coordinate must contain at least two numbers"));
            }

            let longitude = pair[0]
                .as_f64()
                .context("coordinate longitude must be numeric")?;
            let latitude = pair[1]
                .as_f64()
                .context("coordinate latitude must be numeric")?;

            Ok([longitude, latitude])
        })
        .collect::<Result<Vec<_>>>()?;

    if points.is_empty() {
        return Err(anyhow!("LineString coordinates cannot be empty"));
    }

    Ok(points)
}

fn representative_point(segments: &[Vec<[f64; 2]>]) -> Result<(f64, f64)> {
    let first = segments
        .iter()
        .find_map(|segment| segment.first().copied())
        .context("geometry has no coordinates")?;

    let total_length = segments
        .iter()
        .flat_map(|segment| segment.windows(2))
        .map(|window| line_length(window[0], window[1]))
        .sum::<f64>();

    if total_length <= f64::EPSILON {
        return Ok((first[0], first[1]));
    }

    let target = total_length / 2.0;
    let mut accumulated = 0.0;

    for segment in segments {
        for window in segment.windows(2) {
            let start = window[0];
            let end = window[1];
            let length = line_length(start, end);

            if length <= f64::EPSILON {
                continue;
            }

            if accumulated + length >= target {
                let ratio = (target - accumulated) / length;
                return Ok((
                    start[0] + ((end[0] - start[0]) * ratio),
                    start[1] + ((end[1] - start[1]) * ratio),
                ));
            }

            accumulated += length;
        }
    }

    let last = segments
        .iter()
        .rev()
        .find_map(|segment| segment.last().copied())
        .context("geometry has no terminal coordinate")?;

    Ok((last[0], last[1]))
}

fn geometry_geojson_string(segments: &[Vec<[f64; 2]>]) -> Result<String> {
    let geometry = if segments.len() == 1 {
        json!({
            "type": "LineString",
            "coordinates": segments[0],
        })
    } else {
        json!({
            "type": "MultiLineString",
            "coordinates": segments,
        })
    };

    serde_json::to_string(&geometry).map_err(Into::into)
}

fn build_station_uid(key: &LogicalStationKey) -> String {
    let anchor = if !key.source_group_code.is_empty() {
        key.source_group_code.as_str()
    } else if !key.source_station_code.is_empty() {
        key.source_station_code.as_str()
    } else {
        "anon"
    };

    let digest_material = format!(
        "{}|{}|{}|{}",
        anchor, key.operator_name, key.line_name, key.station_name
    );
    let digest = sha256_hex(digest_material.as_bytes());

    format!(
        "{STATION_UID_PREFIX}{}_{}",
        sanitize_code(anchor),
        &digest[..16]
    )
}

fn sanitize_code(input: &str) -> String {
    let sanitized = input
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();

    if sanitized.is_empty() {
        "anon".to_string()
    } else {
        sanitized
    }
}

fn normalized_text(value: String) -> String {
    value.trim().to_string()
}

fn optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn created_detail_json(station: &ParsedStation) -> Value {
    json!({
        "station_name": station.station_name,
        "line_name": station.line_name,
        "operator_name": station.operator_name,
        "source_station_code": station.source_station_code,
        "source_group_code": station.source_group_code,
    })
}

fn updated_detail_json(before: &ExistingVersion, after: &ParsedStation) -> Value {
    let mut changed_fields = Vec::new();

    if before.station_name != after.station_name {
        changed_fields.push("station_name");
    }
    if before.line_name != after.line_name {
        changed_fields.push("line_name");
    }
    if before.operator_name != after.operator_name {
        changed_fields.push("operator_name");
    }
    if before.source_station_code.as_deref() != after.source_station_code.as_deref() {
        changed_fields.push("source_station_code");
    }
    if before.source_group_code.as_deref() != after.source_group_code.as_deref() {
        changed_fields.push("source_group_code");
    }
    if before.status != after.status {
        changed_fields.push("status");
    }
    if (before.latitude - after.latitude).abs() > f64::EPSILON
        || (before.longitude - after.longitude).abs() > f64::EPSILON
    {
        changed_fields.push("representative_point");
    }
    if before.geometry_geojson.as_deref() != Some(after.geometry_geojson.as_str()) {
        changed_fields.push("geometry_geojson");
    }

    json!({
        "changed_fields": changed_fields,
        "before": {
            "station_name": before.station_name,
            "line_name": before.line_name,
            "operator_name": before.operator_name,
            "source_station_code": before.source_station_code,
            "source_group_code": before.source_group_code,
            "status": before.status,
        },
        "after": {
            "station_name": after.station_name,
            "line_name": after.line_name,
            "operator_name": after.operator_name,
            "source_station_code": after.source_station_code,
            "source_group_code": after.source_group_code,
            "status": after.status,
        }
    })
}

fn removed_detail_json(stale: &ExistingVersion) -> Value {
    json!({
        "station_name": stale.station_name,
        "line_name": stale.line_name,
        "operator_name": stale.operator_name,
        "source_station_code": stale.source_station_code,
        "source_group_code": stale.source_group_code,
    })
}

fn line_length(start: [f64; 2], end: [f64; 2]) -> f64 {
    let delta_x = end[0] - start[0];
    let delta_y = end[1] - start[1];

    (delta_x * delta_x + delta_y * delta_y).sqrt()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use sqlx::{any::AnyPoolOptions, Row};
    use station_shared::db::ensure_sqlx_drivers;
    use zip::{write::SimpleFileOptions, ZipWriter};

    #[test]
    fn sqlite_chunk_defaults_stay_within_bind_limit() {
        let chunk_config = PersistChunkConfig::for_dialect(SqlDialect::Sqlite);

        assert_eq!(chunk_config.write_chunk_size, SQLITE_SAFE_WRITE_CHUNK_SIZE);
        assert_eq!(chunk_config.close_chunk_size, SQLITE_SAFE_CLOSE_CHUNK_SIZE);
    }

    #[test]
    fn sqlite_chunk_overrides_are_clamped() {
        let chunk_config = PersistChunkConfig {
            write_chunk_size: 1_000,
            close_chunk_size: 1_000,
        }
        .clamp_for_dialect(SqlDialect::Sqlite);

        assert_eq!(chunk_config.write_chunk_size, SQLITE_SAFE_WRITE_CHUNK_SIZE);
        assert_eq!(chunk_config.close_chunk_size, SQLITE_SAFE_CLOSE_CHUNK_SIZE);
    }

    #[test]
    fn zero_chunk_sizes_are_rejected() {
        let write_error = PersistChunkConfig {
            write_chunk_size: 0,
            close_chunk_size: 1,
        }
        .validate()
        .unwrap_err();
        let close_error = PersistChunkConfig {
            write_chunk_size: 1,
            close_chunk_size: 0,
        }
        .validate()
        .unwrap_err();

        assert!(write_error
            .to_string()
            .contains("write_chunk_size must be greater than 0"));
        assert!(close_error
            .to_string()
            .contains("close_chunk_size must be greater than 0"));
    }

    #[test]
    fn groups_duplicate_segments_into_one_station() {
        let sample = r#"{
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
                "coordinates": [[139.699, 35.690], [139.700, 35.691]]
              }
            },
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
                "coordinates": [[139.700, 35.691], [139.701, 35.692]]
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
        }"#;
        let parsed =
            parse_feature_collection(sample.as_bytes(), Some("N02-24".to_string())).unwrap();

        assert_eq!(parsed.parsed_features, 3);
        assert_eq!(parsed.stations.len(), 2);
        assert_eq!(parsed.source_version.as_deref(), Some("N02-24"));
        assert!(parsed
            .stations
            .iter()
            .any(|station| station.station_name == "新宿" && station.line_name == "京王線"));
    }

    #[test]
    fn calculates_line_midpoint_for_representative_point() {
        let point = representative_point(&[vec![[139.0, 35.0], [141.0, 35.0]]]).unwrap();
        assert!((point.0 - 140.0).abs() < 1e-6);
        assert!((point.1 - 35.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn ingest_snapshot_is_idempotent_for_identical_zip_bytes() {
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

        let first = ingest_snapshot(
            &pool,
            SqlDialect::Sqlite,
            "file:///tmp/N02-24_GML.zip",
            "/tmp/first.zip",
            &zip_bytes,
        )
        .await
        .unwrap();
        let second = ingest_snapshot(
            &pool,
            SqlDialect::Sqlite,
            "file:///tmp/N02-24_GML.zip",
            "/tmp/second.zip",
            &zip_bytes,
        )
        .await
        .unwrap();

        assert_eq!(first.created, 2);
        assert_eq!(first.updated, 0);
        assert_eq!(first.removed, 0);
        assert!(!first.skipped_existing_snapshot);

        assert_eq!(second.created, 0);
        assert_eq!(second.updated, 0);
        assert_eq!(second.removed, 0);
        assert!(second.skipped_existing_snapshot);
        assert_snapshot_phase_timings_are_sane(&first);
        assert_snapshot_phase_timings_are_sane(&second);
        assert_eq!(second.diff_ms, 0);

        assert_eq!(
            query_count(&pool, "SELECT COUNT(*) AS count FROM source_snapshots").await,
            1
        );
        assert_eq!(
            query_count(&pool, "SELECT COUNT(*) AS count FROM station_versions").await,
            2
        );
        assert_eq!(
            query_count(&pool, "SELECT COUNT(*) AS count FROM stations_latest").await,
            2
        );
        assert_eq!(
            query_count(&pool, "SELECT COUNT(*) AS count FROM station_change_events").await,
            2
        );
    }

    #[tokio::test]
    async fn ingest_snapshot_tracks_created_updated_and_removed_stations() {
        let pool = test_pool().await;
        let first_zip = snapshot_zip_bytes(
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
        let second_zip = snapshot_zip_bytes(
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
                    "coordinates": [[139.699, 35.690], [139.703, 35.694]]
                  }
                },
                {
                  "properties": {
                    "N02_003": "山手線",
                    "N02_004": "東日本旅客鉄道",
                    "N02_005": "渋谷",
                    "N02_005c": "003620",
                    "N02_005g": "003620"
                  },
                  "geometry": {
                    "type": "LineString",
                    "coordinates": [[139.700, 35.657], [139.702, 35.659]]
                  }
                }
              ]
            }"#,
        );

        let first = ingest_snapshot(
            &pool,
            SqlDialect::Sqlite,
            "file:///tmp/N02-24_GML.zip",
            "/tmp/first.zip",
            &first_zip,
        )
        .await
        .unwrap();
        let second = ingest_snapshot(
            &pool,
            SqlDialect::Sqlite,
            "file:///tmp/N02-25_GML.zip",
            "/tmp/second.zip",
            &second_zip,
        )
        .await
        .unwrap();

        assert_eq!(first.created, 2);
        assert_eq!(second.created, 1);
        assert_eq!(second.updated, 1);
        assert_eq!(second.removed, 1);
        assert_snapshot_phase_timings_are_sane(&second);
        assert_eq!(
            query_count(&pool, "SELECT COUNT(*) AS count FROM stations_latest").await,
            2
        );

        let latest_shinjuku = sqlx::query(
            "SELECT latitude, longitude
             FROM stations_latest
             WHERE station_name = '新宿'
             LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!((latest_shinjuku.try_get::<f64, _>("latitude").unwrap() - 35.692).abs() < 1e-6);
        assert!((latest_shinjuku.try_get::<f64, _>("longitude").unwrap() - 139.701).abs() < 1e-6);

        assert_eq!(
            query_count(
                &pool,
                "SELECT COUNT(*) AS count FROM station_change_events WHERE change_kind = 'created'",
            )
            .await,
            3
        );
        assert_eq!(
            query_count(
                &pool,
                "SELECT COUNT(*) AS count FROM station_change_events WHERE change_kind = 'updated'",
            )
            .await,
            1
        );
        assert_eq!(
            query_count(
                &pool,
                "SELECT COUNT(*) AS count FROM station_change_events WHERE change_kind = 'removed'",
            )
            .await,
            1
        );
    }

    fn assert_snapshot_phase_timings_are_sane(report: &IngestReport) {
        let report_json = serde_json::to_value(report).unwrap();
        let total_ms = phase_timing_value(&report_json, "total_ms");
        let load_ms = phase_timing_value(&report_json, "load_ms");
        let save_zip_ms = phase_timing_value(&report_json, "save_zip_ms");
        let component_keys = ["extract_ms", "parse_ms", "diff_ms", "persist_ms"];
        let component_sum = component_keys
            .iter()
            .map(|key| {
                let value = phase_timing_value(&report_json, key);
                assert!(value <= total_ms, "{key} should not exceed total_ms");
                value
            })
            .sum::<u64>();

        assert_eq!(load_ms, 0);
        assert_eq!(save_zip_ms, 0);
        assert!(
            total_ms >= component_sum,
            "total_ms should cover snapshot phase timings"
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

    async fn query_count(pool: &AnyPool, sql: &str) -> i64 {
        sqlx::query(sql)
            .fetch_one(pool)
            .await
            .unwrap()
            .try_get::<i64, _>("count")
            .unwrap()
    }

    fn snapshot_zip_bytes(geojson: &str) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(STATION_GEOJSON_PATH, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(geojson.as_bytes()).unwrap();
        writer.finish().unwrap().into_inner()
    }
}
