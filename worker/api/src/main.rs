mod error;
mod openapi;
mod schema;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use sqlx::{AnyPool, Row};
use station_shared::{
    config::AppConfig,
    db::{
        connect_any_pool, decode_optional_string, decode_required_string, distinct_text_count_sql,
        integer_aggregate_sql, prefix_scope_arg, prefix_scope_sql, SqlDialect,
    },
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn};

use crate::{
    error::{internal_error, ApiError, ApiQuery, ApiResult},
    schema::{
        ApiErrorResponseDto, DatasetChangeDetailDto, DatasetChangeEventDto, DatasetChangeKindDto,
        DatasetChangesParams, DatasetChangesResponseDto, DatasetSnapshotChangeCountsDto,
        DatasetSnapshotDto, DatasetSnapshotRefDto, DatasetSnapshotsParams,
        DatasetSnapshotsResponseDto, DatasetStatusResponseDto, HealthResponseDto,
        LineCatalogItemDto, LineCatalogParams, LineCatalogResponseDto, LineStationsParams,
        LineStationsResponseDto, NearbyParams, NearbyStationsQueryDto, NearbyStationsResponseDto,
        OperatorStationsResponseDto, ReadinessDatasetDto, ReadinessResponseDto, SearchParams,
        StationSearchResponseDto, StationSummaryDto,
    },
};

const FULL_DATASET_MIN_STATION_COUNT: i64 = 10_000;
const N02_SOURCE_NAME: &str = "ksj_n02_station";
const N02_STATION_UID_PREFIX: &str = "stn_n02_";
const DUMP_OPENAPI_JSON_FLAG: &str = "--dump-openapi-json";

#[derive(Clone)]
struct AppState {
    config: AppConfig,
    dialect: SqlDialect,
    pool: AnyPool,
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
        .route("/v1/dataset/status", get(dataset_status))
        .route("/v1/dataset/snapshots", get(dataset_snapshots))
        .route("/v1/dataset/changes", get(dataset_changes))
        .route("/v1/stations/search", get(search_stations))
        .route("/v1/stations/nearby", get(nearby_stations))
        .route("/v1/lines/catalog", get(line_catalog))
        .route("/v1/lines/{line_name}/stations", get(line_stations))
        .route(
            "/v1/operators/{operator_name}/stations",
            get(operator_stations),
        )
        .merge(openapi::docs_router())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[tokio::main]
async fn main() {
    if std::env::args().any(|arg| arg == DUMP_OPENAPI_JSON_FLAG) {
        let openapi_json =
            serde_json::to_string_pretty(&<openapi::ApiDoc as utoipa::OpenApi>::openapi())
                .expect("failed to serialize openapi");
        println!("{openapi_json}");
        return;
    }

    tracing_subscriber::fmt().with_env_filter("info").init();

    let config = AppConfig::from_env("station-api").expect("failed to load config");
    let pool = connect_any_pool(&config.database_url)
        .await
        .expect("failed to connect database");
    let state = AppState {
        config: config.clone(),
        dialect: SqlDialect::from(&config.database_type),
        pool,
    };
    let app = app(state);

    info!("starting {} on {}", config.service_name, config.bind_addr);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .expect("bind failed");

    axum::serve(listener, app).await.expect("server failed");
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "station-api",
    responses(
        (status = 200, description = "Liveness check.", body = HealthResponseDto)
    )
)]
async fn health(State(state): State<AppState>) -> Json<HealthResponseDto> {
    Json(HealthResponseDto {
        status: "ok".to_string(),
        service: state.config.service_name,
    })
}

#[utoipa::path(
    get,
    path = "/ready",
    tag = "station-api",
    responses(
        (status = 200, description = "Readiness check succeeded.", body = ReadinessResponseDto),
        (status = 503, description = "Readiness check failed.", body = ReadinessResponseDto)
    )
)]
async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let cache = match (&state.config.redis_url, state.config.ready_require_cache) {
        (Some(_), true) => "required",
        (Some(_), false) => "optional",
        (None, _) => "disabled",
    };

    match sqlx::query(&state.dialect.statement("SELECT 1"))
        .execute(&state.pool)
        .await
    {
        Ok(_) => {
            let dataset = readiness_dataset_summary(&state)
                .await
                .unwrap_or_else(|err| {
                    error!(error = ?err, "dataset readiness summary failed");
                    unknown_readiness_dataset()
                });
            Json(readiness_response(&state, "ready", cache, dataset)).into_response()
        }
        Err(err) => {
            error!(error = %err, "database readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(readiness_response(
                    &state,
                    "not_ready",
                    cache,
                    unknown_readiness_dataset(),
                )),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/metrics",
    tag = "station-api",
    responses(
        (status = 200, description = "Prometheus metrics in text exposition format.", content_type = "text/plain", body = String)
    )
)]
async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let dataset = match fetch_n02_dataset_aggregates(&state).await {
        Ok(dataset) => Some(dataset),
        Err(err) => {
            warn!(error = ?err, "metrics dataset summary failed");
            None
        }
    };

    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        render_prometheus_metrics(&state, dataset.as_ref()),
    )
}

#[derive(Debug)]
struct N02DatasetAggregates {
    active_station_count: i64,
    distinct_station_name_count: i64,
    distinct_line_count: i64,
    active_version_snapshot_count: i64,
    latest_snapshot_id: i64,
}

async fn fetch_n02_dataset_aggregates(state: &AppState) -> Result<N02DatasetAggregates, ApiError> {
    let station_uid_scope =
        prefix_scope_sql(state.dialect, "station_uid", N02_STATION_UID_PREFIX.len());
    let sql = format!(
        "SELECT
           {} AS active_station_count,
           {} AS distinct_station_name_count,
           {} AS distinct_line_count,
           {} AS active_version_snapshot_count,
           {} AS latest_snapshot_id
         FROM stations_latest
         WHERE {station_uid_scope}",
        integer_aggregate_sql(state.dialect, "COUNT(*)"),
        integer_aggregate_sql(
            state.dialect,
            &distinct_text_count_sql(state.dialect, "station_name"),
        ),
        integer_aggregate_sql(
            state.dialect,
            &distinct_text_count_sql(state.dialect, "line_name"),
        ),
        integer_aggregate_sql(state.dialect, "COUNT(DISTINCT snapshot_id)"),
        integer_aggregate_sql(state.dialect, "MAX(snapshot_id)"),
    );
    let row = sqlx::query(&state.dialect.statement(&sql))
        .bind(prefix_scope_arg(state.dialect, N02_STATION_UID_PREFIX))
        .fetch_one(&state.pool)
        .await
        .map_err(internal_error)?;

    Ok(N02DatasetAggregates {
        active_station_count: row
            .try_get::<i64, _>("active_station_count")
            .map_err(internal_error)?,
        distinct_station_name_count: row
            .try_get::<i64, _>("distinct_station_name_count")
            .map_err(internal_error)?,
        distinct_line_count: row
            .try_get::<i64, _>("distinct_line_count")
            .map_err(internal_error)?,
        active_version_snapshot_count: row
            .try_get::<i64, _>("active_version_snapshot_count")
            .map_err(internal_error)?,
        latest_snapshot_id: row
            .try_get::<i64, _>("latest_snapshot_id")
            .map_err(internal_error)?,
    })
}

fn render_prometheus_metrics(state: &AppState, dataset: Option<&N02DatasetAggregates>) -> String {
    let service = prometheus_label_value(&state.config.service_name);
    let database_type = prometheus_label_value(&state.config.database_type.to_string());
    let database_up = i64::from(dataset.is_some());
    let mut body = format!(
        "# HELP station_api_up station-api process is serving requests.\n\
         # TYPE station_api_up gauge\n\
         station_api_up{{service=\"{service}\"}} 1\n\
         # HELP station_api_database_up Backing database query status for metrics collection.\n\
         # TYPE station_api_database_up gauge\n\
         station_api_database_up{{database_type=\"{database_type}\"}} {database_up}\n",
    );

    if let Some(dataset) = dataset {
        body.push_str(&format!(
            "# HELP station_api_n02_active_station_count Active station rows from the canonical N02 source.\n\
             # TYPE station_api_n02_active_station_count gauge\n\
             station_api_n02_active_station_count {}\n\
             # HELP station_api_n02_distinct_station_name_count Distinct active station names from the canonical N02 source.\n\
             # TYPE station_api_n02_distinct_station_name_count gauge\n\
             station_api_n02_distinct_station_name_count {}\n\
             # HELP station_api_n02_distinct_line_count Distinct active line names from the canonical N02 source.\n\
             # TYPE station_api_n02_distinct_line_count gauge\n\
             station_api_n02_distinct_line_count {}\n\
             # HELP station_api_n02_active_version_snapshot_count Source snapshots represented by active N02 station versions.\n\
             # TYPE station_api_n02_active_version_snapshot_count gauge\n\
             station_api_n02_active_version_snapshot_count {}\n\
             # HELP station_api_n02_latest_snapshot_id Latest source snapshot id represented by active N02 station versions, or 0 when empty.\n\
             # TYPE station_api_n02_latest_snapshot_id gauge\n\
             station_api_n02_latest_snapshot_id {}\n",
            dataset.active_station_count,
            dataset.distinct_station_name_count,
            dataset.distinct_line_count,
            dataset.active_version_snapshot_count,
            dataset.latest_snapshot_id,
        ));
    }

    body
}

fn prometheus_label_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}

fn readiness_response(
    state: &AppState,
    status: &str,
    cache: &str,
    dataset: ReadinessDatasetDto,
) -> ReadinessResponseDto {
    ReadinessResponseDto {
        status: status.to_string(),
        database_type: state.config.database_type.to_string(),
        cache: cache.to_string(),
        dataset,
    }
}

fn unknown_readiness_dataset() -> ReadinessDatasetDto {
    ReadinessDatasetDto {
        status: "unknown".to_string(),
        active_station_count: None,
        active_snapshot_id: None,
    }
}

async fn readiness_dataset_summary(state: &AppState) -> Result<ReadinessDatasetDto, ApiError> {
    let station_uid_scope =
        prefix_scope_sql(state.dialect, "station_uid", N02_STATION_UID_PREFIX.len());
    let count_expr = integer_aggregate_sql(state.dialect, "COUNT(*)");
    let sql = format!(
        "SELECT
           {count_expr} AS active_station_count,
           MAX(snapshot_id) AS active_snapshot_id
         FROM stations_latest
         WHERE {station_uid_scope}",
    );
    let row = sqlx::query(&state.dialect.statement(&sql))
        .bind(prefix_scope_arg(state.dialect, N02_STATION_UID_PREFIX))
        .fetch_one(&state.pool)
        .await
        .map_err(internal_error)?;
    let active_station_count = row
        .try_get::<i64, _>("active_station_count")
        .map_err(internal_error)?;
    let active_snapshot_id = row
        .try_get::<Option<i64>, _>("active_snapshot_id")
        .map_err(internal_error)?;

    Ok(ReadinessDatasetDto {
        status: if active_station_count >= FULL_DATASET_MIN_STATION_COUNT {
            "ready".to_string()
        } else {
            "needs_ingest".to_string()
        },
        active_station_count: Some(active_station_count),
        active_snapshot_id,
    })
}

#[utoipa::path(
    get,
    path = "/v1/stations/search",
    tag = "station-api",
    params(SearchParams),
    responses(
        (status = 200, description = "Station search results.", body = StationSearchResponseDto),
        (status = 400, description = "Invalid query parameters.", body = ApiErrorResponseDto),
        (status = 500, description = "Internal server error.", body = ApiErrorResponseDto)
    )
)]
async fn search_stations(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<SearchParams>,
) -> ApiResult<StationSearchResponseDto> {
    let query = params.q.unwrap_or_default().trim().to_string();
    let limit = normalized_limit(params.limit);

    if query.is_empty() {
        return Ok(Json(StationSearchResponseDto {
            items: Vec::new(),
            limit,
            query,
        }));
    }

    let like = format!("%{query}%");
    let prefix = format!("{query}%");
    let station_name_like = text_like_sql(state.dialect, "station_name");
    let station_name_exact = text_equals_sql(state.dialect, "station_name");
    let station_name_prefix = text_like_sql(state.dialect, "station_name");
    let operator_name_order = text_order_sql(state.dialect, "operator_name");
    let line_name_order = text_order_sql(state.dialect, "line_name");
    let station_name_order = text_order_sql(state.dialect, "station_name");
    let sql = format!(
        "SELECT
           station_uid,
           station_name,
           line_name,
           operator_name,
           latitude,
           longitude,
           status
         FROM stations_latest
         WHERE {station_name_like}
         ORDER BY
           CASE
             WHEN {station_name_exact} THEN 0
             WHEN {station_name_prefix} THEN 1
             ELSE 2
           END,
           {operator_name_order},
           {line_name_order},
           {station_name_order}
         LIMIT ?",
    );
    let rows = sqlx::query(&state.dialect.statement(&sql))
        .bind(&like)
        .bind(&query)
        .bind(&prefix)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
        .map_err(internal_error)?;

    let items = rows
        .into_iter()
        .map(row_to_station_summary)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;

    Ok(Json(StationSearchResponseDto {
        items,
        limit,
        query,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/dataset/status",
    tag = "station-api",
    responses(
        (status = 200, description = "Current dataset status.", body = DatasetStatusResponseDto),
        (status = 500, description = "Internal server error.", body = ApiErrorResponseDto)
    )
)]
async fn dataset_status(State(state): State<AppState>) -> ApiResult<DatasetStatusResponseDto> {
    let counts = fetch_n02_dataset_aggregates(&state).await?;
    let active_station_count = counts.active_station_count;
    let distinct_station_name_count = counts.distinct_station_name_count;
    let distinct_line_count = counts.distinct_line_count;
    let active_version_snapshot_count = counts.active_version_snapshot_count;

    let active_snapshot = sqlx::query(&state.dialect.statement(
        "SELECT id, source_version, source_url
         FROM source_snapshots
         WHERE source_name = ?
         ORDER BY id DESC
         LIMIT 1",
    ))
    .bind(N02_SOURCE_NAME)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .map(|row| {
        let id = row.try_get::<i64, _>("id")?;
        let source_version =
            decode_optional_string(&row, "source_version").map_err(map_anyhow_to_sqlx_error)?;
        let source_url =
            decode_required_string(&row, "source_url").map_err(map_anyhow_to_sqlx_error)?;

        Ok::<DatasetSnapshotRefDto, sqlx::Error>(DatasetSnapshotRefDto {
            id,
            source_version,
            source_url,
        })
    });
    let active_snapshot = active_snapshot.transpose().map_err(internal_error)?;

    let source_url = active_snapshot
        .as_ref()
        .map(|snapshot| snapshot.source_url.as_str())
        .unwrap_or_default();
    let source_is_local = !(source_url.is_empty() || is_remote_http_url(source_url));
    let looks_like_full_dataset = active_station_count >= FULL_DATASET_MIN_STATION_COUNT;

    Ok(Json(DatasetStatusResponseDto {
        status: if looks_like_full_dataset {
            "ready".to_string()
        } else {
            "needs_ingest".to_string()
        },
        looks_like_full_dataset,
        source_is_local,
        active_station_count,
        distinct_station_name_count,
        distinct_line_count,
        active_version_snapshot_count,
        active_snapshot,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/dataset/snapshots",
    tag = "station-api",
    params(DatasetSnapshotsParams),
    responses(
        (status = 200, description = "Recent dataset snapshots for the canonical N02 source.", body = DatasetSnapshotsResponseDto),
        (status = 400, description = "Invalid query parameters.", body = ApiErrorResponseDto),
        (status = 500, description = "Internal server error.", body = ApiErrorResponseDto)
    )
)]
async fn dataset_snapshots(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<DatasetSnapshotsParams>,
) -> ApiResult<DatasetSnapshotsResponseDto> {
    let limit = normalized_history_limit(params.limit);
    let downloaded_at_expr = state.dialect.text_cast("ss.downloaded_at");
    let version_prefix_scope =
        prefix_scope_sql(state.dialect, "station_uid", N02_STATION_UID_PREFIX.len());
    let change_prefix_scope =
        prefix_scope_sql(state.dialect, "station_uid", N02_STATION_UID_PREFIX.len());
    let version_count_expr = integer_aggregate_sql(state.dialect, "COUNT(*)");
    let created_count_expr = integer_aggregate_sql(
        state.dialect,
        "SUM(CASE WHEN change_kind = 'created' THEN 1 ELSE 0 END)",
    );
    let updated_count_expr = integer_aggregate_sql(
        state.dialect,
        "SUM(CASE WHEN change_kind = 'updated' THEN 1 ELSE 0 END)",
    );
    let removed_count_expr = integer_aggregate_sql(
        state.dialect,
        "SUM(CASE WHEN change_kind = 'removed' THEN 1 ELSE 0 END)",
    );
    let station_version_count_select =
        integer_aggregate_sql(state.dialect, "version_counts.station_version_count");
    let created_count_select = integer_aggregate_sql(state.dialect, "change_counts.created_count");
    let updated_count_select = integer_aggregate_sql(state.dialect, "change_counts.updated_count");
    let removed_count_select = integer_aggregate_sql(state.dialect, "change_counts.removed_count");
    let sql = format!(
        "SELECT
           ss.id,
           ss.source_name,
           ss.source_kind,
           ss.source_version,
           ss.source_url,
           ss.source_sha256,
           {downloaded_at_expr} AS downloaded_at,
           {station_version_count_select} AS station_version_count,
           {created_count_select} AS created_count,
           {updated_count_select} AS updated_count,
           {removed_count_select} AS removed_count
         FROM source_snapshots AS ss
         LEFT JOIN (
           SELECT
             snapshot_id,
             {version_count_expr} AS station_version_count
           FROM station_versions
           WHERE {version_prefix_scope}
           GROUP BY snapshot_id
         ) AS version_counts
           ON version_counts.snapshot_id = ss.id
         LEFT JOIN (
           SELECT
             snapshot_id,
             {created_count_expr} AS created_count,
             {updated_count_expr} AS updated_count,
             {removed_count_expr} AS removed_count
           FROM station_change_events
           WHERE {change_prefix_scope}
           GROUP BY snapshot_id
         ) AS change_counts
           ON change_counts.snapshot_id = ss.id
         WHERE ss.source_name = ?
         ORDER BY ss.id DESC
         LIMIT ?",
    );
    let rows = sqlx::query(&state.dialect.statement(&sql))
        .bind(prefix_scope_arg(state.dialect, N02_STATION_UID_PREFIX))
        .bind(prefix_scope_arg(state.dialect, N02_STATION_UID_PREFIX))
        .bind(N02_SOURCE_NAME)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
        .map_err(internal_error)?;

    let items = rows
        .into_iter()
        .map(|row| {
            let created = row.try_get::<i64, _>("created_count")?;
            let updated = row.try_get::<i64, _>("updated_count")?;
            let removed = row.try_get::<i64, _>("removed_count")?;

            Ok(DatasetSnapshotDto {
                id: row.try_get::<i64, _>("id")?,
                source_name: decode_required_string(&row, "source_name")
                    .map_err(map_anyhow_to_sqlx_error)?,
                source_kind: decode_required_string(&row, "source_kind")
                    .map_err(map_anyhow_to_sqlx_error)?,
                source_version: decode_optional_string(&row, "source_version")
                    .map_err(map_anyhow_to_sqlx_error)?,
                source_url: decode_required_string(&row, "source_url")
                    .map_err(map_anyhow_to_sqlx_error)?,
                source_sha256: decode_required_string(&row, "source_sha256")
                    .map_err(map_anyhow_to_sqlx_error)?,
                downloaded_at: decode_required_string(&row, "downloaded_at")
                    .map_err(map_anyhow_to_sqlx_error)?,
                station_version_count: row.try_get::<i64, _>("station_version_count")?,
                change_counts: DatasetSnapshotChangeCountsDto {
                    created,
                    updated,
                    removed,
                    total: created + updated + removed,
                },
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(internal_error)?;

    Ok(Json(DatasetSnapshotsResponseDto { items, limit }))
}

#[utoipa::path(
    get,
    path = "/v1/dataset/changes",
    tag = "station-api",
    params(DatasetChangesParams),
    responses(
        (status = 200, description = "Recent dataset change events for the canonical N02 source.", body = DatasetChangesResponseDto),
        (status = 400, description = "Invalid query parameters.", body = ApiErrorResponseDto),
        (status = 404, description = "Requested snapshot was not found.", body = ApiErrorResponseDto),
        (status = 500, description = "Internal server error.", body = ApiErrorResponseDto)
    )
)]
async fn dataset_changes(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<DatasetChangesParams>,
) -> ApiResult<DatasetChangesResponseDto> {
    let limit = normalized_history_limit(params.limit);

    if let Some(snapshot_id) = params.snapshot_id {
        ensure_dataset_snapshot_exists(&state, snapshot_id).await?;
    }

    let created_at_expr = state.dialect.text_cast("sce.created_at");
    let station_uid_scope = prefix_scope_sql(
        state.dialect,
        "sce.station_uid",
        N02_STATION_UID_PREFIX.len(),
    );
    let snapshot_filter = if params.snapshot_id.is_some() {
        " AND sce.snapshot_id = ?"
    } else {
        ""
    };
    let sql = format!(
        "SELECT
           sce.id,
           sce.snapshot_id,
           ss.source_version,
           sce.station_uid,
           sce.change_kind,
           sce.before_version_id,
           sce.after_version_id,
           sce.detail_json,
           {created_at_expr} AS created_at,
           COALESCE(after_sv.station_name, before_sv.station_name) AS station_name,
           COALESCE(after_sv.line_name, before_sv.line_name) AS line_name,
           COALESCE(after_sv.operator_name, before_sv.operator_name) AS operator_name
         FROM station_change_events AS sce
         INNER JOIN source_snapshots AS ss
           ON ss.id = sce.snapshot_id
         LEFT JOIN station_versions AS before_sv
           ON before_sv.id = sce.before_version_id
         LEFT JOIN station_versions AS after_sv
           ON after_sv.id = sce.after_version_id
         WHERE ss.source_name = ?
           AND {station_uid_scope}
           {snapshot_filter}
         ORDER BY sce.id DESC
         LIMIT ?",
    );
    let statement = state.dialect.statement(&sql);
    let mut query = sqlx::query(&statement)
        .bind(N02_SOURCE_NAME)
        .bind(prefix_scope_arg(state.dialect, N02_STATION_UID_PREFIX));

    if let Some(snapshot_id) = params.snapshot_id {
        query = query.bind(snapshot_id);
    }

    let rows = query
        .bind(limit)
        .fetch_all(&state.pool)
        .await
        .map_err(internal_error)?;

    let items = rows
        .into_iter()
        .map(|row| {
            let detail = decode_optional_string(&row, "detail_json")
                .map_err(map_anyhow_to_sqlx_error)?
                .map(|detail_json| serde_json::from_str::<DatasetChangeDetailDto>(&detail_json))
                .transpose()
                .map_err(anyhow::Error::from)
                .map_err(map_anyhow_to_sqlx_error)?
                .unwrap_or_default();

            Ok(DatasetChangeEventDto {
                id: row.try_get::<i64, _>("id")?,
                snapshot_id: row.try_get::<i64, _>("snapshot_id")?,
                source_version: decode_optional_string(&row, "source_version")
                    .map_err(map_anyhow_to_sqlx_error)?,
                station_uid: decode_required_string(&row, "station_uid")
                    .map_err(map_anyhow_to_sqlx_error)?,
                change_kind: row_to_change_kind(&row)?,
                station_name: decode_optional_string(&row, "station_name")
                    .map_err(map_anyhow_to_sqlx_error)?,
                line_name: decode_optional_string(&row, "line_name")
                    .map_err(map_anyhow_to_sqlx_error)?,
                operator_name: decode_optional_string(&row, "operator_name")
                    .map_err(map_anyhow_to_sqlx_error)?,
                before_version_id: row.try_get::<Option<i64>, _>("before_version_id")?,
                after_version_id: row.try_get::<Option<i64>, _>("after_version_id")?,
                detail,
                created_at: decode_required_string(&row, "created_at")
                    .map_err(map_anyhow_to_sqlx_error)?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(internal_error)?;

    Ok(Json(DatasetChangesResponseDto {
        items,
        limit,
        snapshot_id: params.snapshot_id,
    }))
}

async fn ensure_dataset_snapshot_exists(
    state: &AppState,
    snapshot_id: i64,
) -> Result<(), ApiError> {
    let exists = sqlx::query(&state.dialect.statement(
        "SELECT id
         FROM source_snapshots
         WHERE id = ? AND source_name = ?
         LIMIT 1",
    ))
    .bind(snapshot_id)
    .bind(N02_SOURCE_NAME)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    if exists.is_some() {
        Ok(())
    } else {
        Err(ApiError::not_found(format!(
            "dataset snapshot {snapshot_id} was not found"
        )))
    }
}

fn is_remote_http_url(url: &str) -> bool {
    let Some((scheme, _)) = url.split_once("://") else {
        return false;
    };

    scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http")
}

#[utoipa::path(
    get,
    path = "/v1/stations/nearby",
    tag = "station-api",
    params(NearbyParams),
    responses(
        (status = 200, description = "Nearby station results.", body = NearbyStationsResponseDto),
        (status = 400, description = "Invalid query parameters.", body = ApiErrorResponseDto),
        (status = 500, description = "Internal server error.", body = ApiErrorResponseDto)
    )
)]
async fn nearby_stations(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<NearbyParams>,
) -> ApiResult<NearbyStationsResponseDto> {
    let limit = normalized_limit(params.limit);
    let rows = sqlx::query(&state.dialect.statement(
        "SELECT
           station_uid,
           station_name,
           line_name,
           operator_name,
           latitude,
           longitude,
           status
         FROM stations_latest
         ORDER BY
           ((latitude - ?) * (latitude - ?)) +
           ((longitude - ?) * (longitude - ?)) ASC,
           station_name ASC
         LIMIT ?",
    ))
    .bind(params.lat)
    .bind(params.lat)
    .bind(params.lng)
    .bind(params.lng)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    let items = rows
        .into_iter()
        .map(row_to_station_summary)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;

    Ok(Json(NearbyStationsResponseDto {
        items,
        limit,
        query: NearbyStationsQueryDto {
            lat: params.lat,
            lng: params.lng,
        },
    }))
}

#[utoipa::path(
    get,
    path = "/v1/lines/catalog",
    tag = "station-api",
    params(LineCatalogParams),
    responses(
        (status = 200, description = "Line catalog search results.", body = LineCatalogResponseDto),
        (status = 400, description = "Invalid query parameters.", body = ApiErrorResponseDto),
        (status = 500, description = "Internal server error.", body = ApiErrorResponseDto)
    )
)]
async fn line_catalog(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<LineCatalogParams>,
) -> ApiResult<LineCatalogResponseDto> {
    let query = params.q.unwrap_or_default().trim().to_string();
    let limit = normalized_catalog_limit(params.limit);
    let line_name_expr = text_group_sql(state.dialect, "line_name");
    let operator_name_expr = text_group_sql(state.dialect, "operator_name");
    let line_name_like = text_like_sql(state.dialect, "line_name");
    let station_count_expr = integer_aggregate_sql(state.dialect, "COUNT(*)");
    let where_clause = if query.is_empty() {
        String::new()
    } else {
        format!("WHERE {line_name_like}")
    };
    let sql = format!(
        "SELECT
           {line_name_expr} AS line_name,
           {operator_name_expr} AS operator_name,
           {station_count_expr} AS station_count
         FROM stations_latest
         {where_clause}
         GROUP BY {line_name_expr}, {operator_name_expr}
         ORDER BY {line_name_expr}, {operator_name_expr}
         LIMIT ?",
    );
    let statement = state.dialect.statement(&sql);
    let rows = if query.is_empty() {
        sqlx::query(&statement)
            .bind(limit)
            .fetch_all(&state.pool)
            .await
    } else {
        let like = format!("%{query}%");
        sqlx::query(&statement)
            .bind(like)
            .bind(limit)
            .fetch_all(&state.pool)
            .await
    }
    .map_err(internal_error)?;

    let items = rows
        .into_iter()
        .map(|row| {
            Ok(LineCatalogItemDto {
                line_name: decode_required_string(&row, "line_name")
                    .map_err(map_anyhow_to_sqlx_error)?,
                operator_name: decode_required_string(&row, "operator_name")
                    .map_err(map_anyhow_to_sqlx_error)?,
                station_count: row.try_get::<i64, _>("station_count")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(internal_error)?;

    Ok(Json(LineCatalogResponseDto {
        items,
        limit,
        query,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/lines/{line_name}/stations",
    tag = "station-api",
    params(
        ("line_name" = String, Path, description = "Exact line name."),
        LineStationsParams
    ),
    responses(
        (status = 200, description = "Stations on the requested line.", body = LineStationsResponseDto),
        (status = 400, description = "Invalid query parameters.", body = ApiErrorResponseDto),
        (status = 500, description = "Internal server error.", body = ApiErrorResponseDto)
    )
)]
async fn line_stations(
    State(state): State<AppState>,
    Path(line_name): Path<String>,
    ApiQuery(params): ApiQuery<LineStationsParams>,
) -> ApiResult<LineStationsResponseDto> {
    let line_name_match = text_equals_sql(state.dialect, "line_name");
    let operator_name_match = text_equals_sql(state.dialect, "operator_name");
    let operator_name_order = text_order_sql(state.dialect, "operator_name");
    let station_name_order = text_order_sql(state.dialect, "station_name");
    let operator_name = params
        .operator_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let where_clause = if operator_name.is_some() {
        format!("{line_name_match} AND {operator_name_match}")
    } else {
        line_name_match
    };
    let sql = format!(
        "SELECT
           station_uid,
           station_name,
           line_name,
           operator_name,
           latitude,
           longitude,
           status
         FROM stations_latest
         WHERE {where_clause}
         ORDER BY {operator_name_order}, {station_name_order}",
    );
    let statement = state.dialect.statement(&sql);
    let rows = if let Some(operator_name) = operator_name.as_deref() {
        sqlx::query(&statement)
            .bind(&line_name)
            .bind(operator_name)
            .fetch_all(&state.pool)
            .await
    } else {
        sqlx::query(&statement)
            .bind(&line_name)
            .fetch_all(&state.pool)
            .await
    }
    .map_err(internal_error)?;

    let items = rows
        .into_iter()
        .map(row_to_station_summary)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;

    Ok(Json(LineStationsResponseDto {
        line_name,
        operator_name,
        items,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/operators/{operator_name}/stations",
    tag = "station-api",
    params(
        ("operator_name" = String, Path, description = "Exact operator name.")
    ),
    responses(
        (status = 200, description = "Stations operated by the requested operator.", body = OperatorStationsResponseDto),
        (status = 500, description = "Internal server error.", body = ApiErrorResponseDto)
    )
)]
async fn operator_stations(
    State(state): State<AppState>,
    Path(operator_name): Path<String>,
) -> ApiResult<OperatorStationsResponseDto> {
    let operator_name_match = text_equals_sql(state.dialect, "operator_name");
    let line_name_order = text_order_sql(state.dialect, "line_name");
    let station_name_order = text_order_sql(state.dialect, "station_name");
    let sql = format!(
        "SELECT
           station_uid,
           station_name,
           line_name,
           operator_name,
           latitude,
           longitude,
           status
         FROM stations_latest
         WHERE {operator_name_match}
         ORDER BY {line_name_order}, {station_name_order}",
    );
    let rows = sqlx::query(&state.dialect.statement(&sql))
        .bind(&operator_name)
        .fetch_all(&state.pool)
        .await
        .map_err(internal_error)?;

    let items = rows
        .into_iter()
        .map(row_to_station_summary)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;

    Ok(Json(OperatorStationsResponseDto {
        operator_name,
        items,
    }))
}

fn normalized_limit(limit: Option<u32>) -> i64 {
    i64::from(limit.unwrap_or(10).clamp(1, 100))
}

fn normalized_catalog_limit(limit: Option<u32>) -> i64 {
    i64::from(limit.unwrap_or(60).clamp(1, 1000))
}

fn normalized_history_limit(limit: Option<u32>) -> i64 {
    i64::from(limit.unwrap_or(20).clamp(1, 200))
}

fn row_to_change_kind(row: &sqlx::any::AnyRow) -> Result<DatasetChangeKindDto, sqlx::Error> {
    match decode_required_string(row, "change_kind")
        .map_err(map_anyhow_to_sqlx_error)?
        .as_str()
    {
        "created" => Ok(DatasetChangeKindDto::Created),
        "updated" => Ok(DatasetChangeKindDto::Updated),
        "removed" => Ok(DatasetChangeKindDto::Removed),
        other => Err(sqlx::Error::Decode(
            anyhow::anyhow!("unsupported change_kind: {other}").into(),
        )),
    }
}

fn row_to_station_summary(row: sqlx::any::AnyRow) -> Result<StationSummaryDto, sqlx::Error> {
    Ok(StationSummaryDto {
        station_uid: decode_required_string(&row, "station_uid")
            .map_err(map_anyhow_to_sqlx_error)?,
        station_name: decode_required_string(&row, "station_name")
            .map_err(map_anyhow_to_sqlx_error)?,
        line_name: decode_required_string(&row, "line_name").map_err(map_anyhow_to_sqlx_error)?,
        operator_name: decode_required_string(&row, "operator_name")
            .map_err(map_anyhow_to_sqlx_error)?,
        latitude: row.try_get("latitude")?,
        longitude: row.try_get("longitude")?,
        status: decode_required_string(&row, "status").map_err(map_anyhow_to_sqlx_error)?,
    })
}

fn map_anyhow_to_sqlx_error(error: anyhow::Error) -> sqlx::Error {
    match error.downcast::<sqlx::Error>() {
        Ok(error) => error,
        Err(error) => sqlx::Error::Decode(error.into()),
    }
}

#[cfg(test)]
fn nullable_integer_aggregate_sql(dialect: SqlDialect, expr: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("CAST({expr} AS SIGNED)"),
        SqlDialect::Postgres | SqlDialect::Sqlite => format!("CAST({expr} AS BIGINT)"),
    }
}

fn text_equals_sql(dialect: SqlDialect, column: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("{column} COLLATE utf8mb4_bin = ?"),
        SqlDialect::Postgres | SqlDialect::Sqlite => format!("{column} = ?"),
    }
}

fn text_like_sql(dialect: SqlDialect, column: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("{column} COLLATE utf8mb4_bin LIKE ?"),
        SqlDialect::Postgres | SqlDialect::Sqlite => format!("{column} LIKE ?"),
    }
}

fn text_order_sql(dialect: SqlDialect, column: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("{column} COLLATE utf8mb4_bin"),
        SqlDialect::Postgres | SqlDialect::Sqlite => column.to_string(),
    }
}

fn text_group_sql(dialect: SqlDialect, column: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("{column} COLLATE utf8mb4_bin"),
        SqlDialect::Postgres | SqlDialect::Sqlite => column.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        extract::{Path, State},
        http::{Request, StatusCode},
        response::IntoResponse,
    };
    use serde_json::Value;
    use sqlx::{any::AnyPoolOptions, AnyPool, Row};
    use station_shared::{
        config::{AppConfig, DatabaseType},
        db::{
            distinct_text_count_sql, ensure_sqlx_drivers, integer_aggregate_sql,
            like_prefix_pattern, prefix_scope_arg, prefix_scope_sql, SqlDialect,
        },
    };
    use tower::util::ServiceExt;

    use super::{
        app, dataset_changes, dataset_snapshots, dataset_status, line_catalog, line_stations,
        map_anyhow_to_sqlx_error, metrics, nullable_integer_aggregate_sql, operator_stations,
        prometheus_label_value, search_stations, text_equals_sql, text_group_sql, text_like_sql,
        text_order_sql, ApiQuery, AppState, DatasetChangeKindDto, DatasetChangesParams,
        DatasetSnapshotsParams, LineCatalogParams, LineStationsParams, SearchParams,
        N02_SOURCE_NAME, N02_STATION_UID_PREFIX,
    };

    #[test]
    fn mysql_distinct_text_count_is_bytewise() {
        assert_eq!(
            distinct_text_count_sql(SqlDialect::Mysql, "line_name"),
            "COUNT(DISTINCT CAST(line_name AS BINARY))"
        );
    }

    #[test]
    fn postgres_distinct_text_count_stays_plain() {
        assert_eq!(
            distinct_text_count_sql(SqlDialect::Postgres, "line_name"),
            "COUNT(DISTINCT line_name)"
        );
    }

    #[test]
    fn mysql_integer_aggregate_casts_to_signed() {
        assert_eq!(
            integer_aggregate_sql(SqlDialect::Mysql, "COUNT(*)"),
            "CAST(COALESCE(COUNT(*), 0) AS SIGNED)"
        );
        assert_eq!(
            nullable_integer_aggregate_sql(SqlDialect::Mysql, "MIN(snapshot_id)"),
            "CAST(MIN(snapshot_id) AS SIGNED)"
        );
    }

    #[test]
    fn mysql_text_filters_and_ordering_use_binary_collation() {
        assert_eq!(
            text_equals_sql(SqlDialect::Mysql, "line_name"),
            "line_name COLLATE utf8mb4_bin = ?"
        );
        assert_eq!(
            text_equals_sql(SqlDialect::Mysql, "operator_name"),
            "operator_name COLLATE utf8mb4_bin = ?"
        );
        assert_eq!(
            text_like_sql(SqlDialect::Mysql, "station_name"),
            "station_name COLLATE utf8mb4_bin LIKE ?"
        );
        assert_eq!(
            text_order_sql(SqlDialect::Mysql, "operator_name"),
            "operator_name COLLATE utf8mb4_bin"
        );
        assert_eq!(
            text_order_sql(SqlDialect::Mysql, "line_name"),
            "line_name COLLATE utf8mb4_bin"
        );
        assert_eq!(
            text_group_sql(SqlDialect::Mysql, "line_name"),
            "line_name COLLATE utf8mb4_bin"
        );
    }

    #[test]
    fn mysql_station_uid_prefix_scope_uses_binary_like_with_escaped_prefix() {
        assert_eq!(
            prefix_scope_sql(
                SqlDialect::Mysql,
                "station_uid",
                N02_STATION_UID_PREFIX.len()
            ),
            "station_uid COLLATE utf8mb4_bin LIKE ? ESCAPE '\\\\'"
        );
        assert_eq!(
            prefix_scope_arg(SqlDialect::Mysql, N02_STATION_UID_PREFIX),
            "stn\\_n02\\_%"
        );
    }

    #[test]
    fn sqlite_station_uid_prefix_scope_stays_substr_exact() {
        assert_eq!(
            prefix_scope_sql(
                SqlDialect::Sqlite,
                "station_uid",
                N02_STATION_UID_PREFIX.len()
            ),
            "substr(station_uid, 1, 8) = ?"
        );
        assert_eq!(
            prefix_scope_arg(SqlDialect::Sqlite, N02_STATION_UID_PREFIX),
            N02_STATION_UID_PREFIX
        );
        assert_eq!(like_prefix_pattern("stn_n02_"), "stn\\_n02\\_%");
    }

    #[test]
    fn map_anyhow_to_sqlx_error_preserves_sqlx_errors() {
        let error = map_anyhow_to_sqlx_error(anyhow::Error::from(sqlx::Error::ColumnNotFound(
            "station_uid".to_string(),
        )));

        assert!(matches!(error, sqlx::Error::ColumnNotFound(name) if name == "station_uid"));
    }

    #[test]
    fn postgres_text_filters_and_ordering_stay_plain() {
        assert_eq!(
            text_equals_sql(SqlDialect::Postgres, "line_name"),
            "line_name = ?"
        );
        assert_eq!(
            text_equals_sql(SqlDialect::Postgres, "operator_name"),
            "operator_name = ?"
        );
        assert_eq!(
            text_like_sql(SqlDialect::Postgres, "station_name"),
            "station_name LIKE ?"
        );
        assert_eq!(
            text_order_sql(SqlDialect::Postgres, "operator_name"),
            "operator_name"
        );
        assert_eq!(
            text_order_sql(SqlDialect::Postgres, "line_name"),
            "line_name"
        );
        assert_eq!(
            text_group_sql(SqlDialect::Postgres, "line_name"),
            "line_name"
        );
    }

    #[tokio::test]
    async fn app_contract_routes_return_expected_shapes() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 1).await;
        let shinjuku_version_id = insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_n02_shinjuku",
                "新宿",
                "山手線",
                "東日本旅客鉄道",
                35.6909,
                139.7003,
            ),
        )
        .await;
        insert_change_event(
            &pool,
            1,
            "stn_n02_shinjuku",
            "created",
            None,
            Some(shinjuku_version_id),
            r#"{"station_name":"新宿","line_name":"山手線","operator_name":"東日本旅客鉄道"}"#,
        )
        .await;

        let app = app(test_state(pool));

        let health = json_body(app.clone().oneshot(get("/health")).await.unwrap()).await;
        assert_eq!(health["status"].as_str(), Some("ok"));

        let ready = json_body(app.clone().oneshot(get("/ready")).await.unwrap()).await;
        assert_eq!(ready["status"].as_str(), Some("ready"));
        assert_eq!(ready["dataset"]["status"].as_str(), Some("needs_ingest"));
        assert_eq!(ready["dataset"]["active_station_count"].as_i64(), Some(1));

        let metrics = app.clone().oneshot(get("/metrics")).await.unwrap();
        assert_eq!(metrics.status(), StatusCode::OK);
        assert!(metrics
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/plain")));
        let metrics = text_body(metrics).await;
        assert!(metrics.contains("station_api_database_up{database_type=\"sqlite\"} 1"));
        assert!(metrics.contains("station_api_n02_active_station_count 1"));

        let dataset = json_body(
            app.clone()
                .oneshot(get("/v1/dataset/status"))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(dataset["active_station_count"].as_i64(), Some(1));

        let search = json_body(
            app.clone()
                .oneshot(get("/v1/stations/search?q=%E6%96%B0%E5%AE%BF&limit=1"))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(search["items"].as_array().map(Vec::len), Some(1));

        let snapshots = json_body(
            app.clone()
                .oneshot(get("/v1/dataset/snapshots?limit=1"))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(snapshots["items"].as_array().map(Vec::len), Some(1));

        let changes = json_body(
            app.clone()
                .oneshot(get("/v1/dataset/changes?limit=1"))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(changes["items"].as_array().map(Vec::len), Some(1));

        let nearby = json_body(
            app.oneshot(get("/v1/stations/nearby?lat=35.6909&lng=139.7003&limit=1"))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(nearby["items"].as_array().map(Vec::len), Some(1));
    }

    #[tokio::test]
    async fn openapi_json_lists_current_public_paths() {
        let app = app(test_state(test_pool().await));
        let response = app.oneshot(get("/openapi.json")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = json_body(response).await;
        let paths = body["paths"].as_object().unwrap();

        for path in [
            "/health",
            "/ready",
            "/metrics",
            "/v1/dataset/status",
            "/v1/dataset/snapshots",
            "/v1/dataset/changes",
            "/v1/stations/search",
            "/v1/stations/nearby",
            "/v1/lines/catalog",
            "/v1/lines/{line_name}/stations",
            "/v1/operators/{operator_name}/stations",
        ] {
            assert!(paths.contains_key(path), "missing path {path}");
        }
    }

    #[tokio::test]
    async fn openapi_json_excludes_frontend_helper_routes() {
        let app = app(test_state(test_pool().await));
        let response = app.oneshot(get("/openapi.json")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = json_body(response).await;
        let paths = body["paths"].as_object().unwrap();

        assert!(!paths.contains_key("/api/address-search"));
    }

    #[tokio::test]
    async fn openapi_json_documents_line_catalog_limit_up_to_1000() {
        let app = app(test_state(test_pool().await));
        let response = app.oneshot(get("/openapi.json")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = json_body(response).await;
        let parameters = body["paths"]["/v1/lines/catalog"]["get"]["parameters"]
            .as_array()
            .unwrap();
        let limit = parameters
            .iter()
            .find(|parameter| parameter["name"].as_str() == Some("limit"))
            .unwrap();

        assert_eq!(limit["schema"]["minimum"].as_i64(), Some(1));
        assert_eq!(limit["schema"]["maximum"].as_i64(), Some(1000));
    }

    #[tokio::test]
    async fn openapi_json_documents_history_and_error_contract() {
        let app = app(test_state(test_pool().await));
        let response = app.oneshot(get("/openapi.json")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = json_body(response).await;
        let schemas = body["components"]["schemas"].as_object().unwrap();

        assert_eq!(
            schemas["ApiErrorResponseDto"]["properties"]["error"]["description"].as_str(),
            Some("Standard error envelope for public station-api endpoints.")
        );
        assert!(schemas.contains_key("ApiErrorDetailPayloadDto"));
        assert!(schemas.contains_key("ApiErrorIssueDto"));
        assert_eq!(
            schemas["ApiErrorDetailPayloadDto"]["properties"]["kind"]["description"].as_str(),
            Some("Stable category for the detail payload.")
        );
        assert_eq!(
            schemas["ApiErrorDetailPayloadDto"]["properties"]["issues"]["description"].as_str(),
            Some("One or more issues associated with the error.")
        );
        assert_eq!(
            schemas["ReadinessResponseDto"]["properties"]["dataset"]["description"].as_str(),
            Some("Dataset readiness summary for the canonical N02 station rows.")
        );
        let active_snapshot = &schemas["DatasetStatusResponseDto"]["properties"]["active_snapshot"];
        let active_snapshot_description = active_snapshot["description"].as_str().or_else(|| {
            active_snapshot["oneOf"].as_array().and_then(|schemas| {
                schemas
                    .iter()
                    .find_map(|schema| schema["description"].as_str())
            })
        });
        assert_eq!(
            active_snapshot_description,
            Some("Latest ingested N02 source snapshot metadata, when one exists.")
        );
        assert_eq!(
            schemas["DatasetChangeEventDto"]["properties"]["detail"]["description"].as_str(),
            Some("Structured before/after context for consumers that need field-level diffs.")
        );

        let snapshot_parameters = body["paths"]["/v1/dataset/snapshots"]["get"]["parameters"]
            .as_array()
            .unwrap();
        let snapshot_limit = snapshot_parameters
            .iter()
            .find(|parameter| parameter["name"].as_str() == Some("limit"))
            .unwrap();
        assert_eq!(
            snapshot_limit["description"].as_str(),
            Some("Maximum number of recent N02 source snapshots to return.")
        );

        let change_parameters = body["paths"]["/v1/dataset/changes"]["get"]["parameters"]
            .as_array()
            .unwrap();
        let snapshot_id = change_parameters
            .iter()
            .find(|parameter| parameter["name"].as_str() == Some("snapshot_id"))
            .unwrap();
        assert_eq!(
            snapshot_id["description"].as_str(),
            Some("Optional source snapshot id to filter change events.")
        );
    }

    #[tokio::test]
    async fn docs_entrypoint_redirects_and_html_is_served() {
        let app = app(test_state(test_pool().await));

        let redirect = app.clone().oneshot(get("/docs")).await.unwrap();
        assert_eq!(redirect.status(), StatusCode::SEE_OTHER);

        let docs = app.oneshot(get("/docs/")).await.unwrap();
        assert_eq!(docs.status(), StatusCode::OK);
        assert!(docs
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/html")));

        let body = text_body(docs).await;
        assert!(body.contains("Swagger UI"));
    }

    #[tokio::test]
    async fn metrics_scopes_counts_to_n02_rows() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 24).await;
        insert_snapshot(&pool, 25).await;
        insert_station(
            &pool,
            24,
            StationSeed::new(
                "stn_n02_shinjuku",
                "新宿",
                "中央線",
                "東日本旅客鉄道",
                35.6900,
                139.7000,
            ),
        )
        .await;
        insert_station(
            &pool,
            25,
            StationSeed::new(
                "stn_n02_shibuya",
                "渋谷",
                "山手線",
                "東日本旅客鉄道",
                35.6580,
                139.7016,
            ),
        )
        .await;
        insert_station(
            &pool,
            25,
            StationSeed::new(
                "stn_other_network",
                "ノイズ駅",
                "ノイズ線",
                "ノイズ交通",
                35.6000,
                139.6000,
            ),
        )
        .await;

        let response = metrics(State(test_state(pool))).await.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = text_body(response).await;
        assert!(body.contains("station_api_n02_active_station_count 2"));
        assert!(body.contains("station_api_n02_distinct_station_name_count 2"));
        assert!(body.contains("station_api_n02_distinct_line_count 2"));
        assert!(body.contains("station_api_n02_active_version_snapshot_count 2"));
        assert!(body.contains("station_api_n02_latest_snapshot_id 25"));
    }

    #[tokio::test]
    async fn metrics_reports_database_up_for_empty_dataset() {
        let response = metrics(State(test_state(test_pool().await)))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = text_body(response).await;
        assert!(body.contains("station_api_database_up{database_type=\"sqlite\"} 1"));
        assert!(body.contains("station_api_n02_active_station_count 0"));
        assert!(body.contains("station_api_n02_latest_snapshot_id 0"));
    }

    #[test]
    fn prometheus_label_values_are_escaped() {
        assert_eq!(
            prometheus_label_value("station-api\"test\\service\nnext"),
            "station-api\\\"test\\\\service\\nnext"
        );
    }

    #[tokio::test]
    async fn invalid_query_parameters_return_standard_error_shape() {
        let app = app(test_state(test_pool().await));
        let response = app
            .oneshot(get("/v1/stations/nearby?lat=abc&lng=139.7003"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = json_body(response).await;
        assert_eq!(body["error"]["code"].as_str(), Some("invalid_request"));
        assert!(body["error"]["message"].as_str().is_some());
        assert_eq!(
            body["error"]["detail"]["kind"].as_str(),
            Some("query_parameters")
        );
        assert!(body["error"]["detail"]["issues"][0]["message"]
            .as_str()
            .is_some());
    }

    #[tokio::test]
    async fn dataset_snapshots_returns_recent_history_with_counts() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 24).await;
        insert_snapshot(&pool, 25).await;

        let old_version_id = insert_station(
            &pool,
            24,
            StationSeed::new("stn_n02_old", "旧駅", "旧線", "旧交通", 35.6000, 139.6000),
        )
        .await;
        let new_version_id = insert_station(
            &pool,
            25,
            StationSeed::new("stn_n02_new", "新駅", "新線", "新交通", 35.7000, 139.7000),
        )
        .await;

        insert_change_event(
            &pool,
            24,
            "stn_n02_old",
            "created",
            None,
            Some(old_version_id),
            r#"{"station_name":"旧駅","line_name":"旧線","operator_name":"旧交通"}"#,
        )
        .await;
        insert_change_event(
            &pool,
            25,
            "stn_n02_new",
            "created",
            None,
            Some(new_version_id),
            r#"{"station_name":"新駅","line_name":"新線","operator_name":"新交通"}"#,
        )
        .await;
        insert_change_event(
            &pool,
            25,
            "stn_n02_old",
            "removed",
            Some(old_version_id),
            None,
            r#"{"station_name":"旧駅","line_name":"旧線","operator_name":"旧交通"}"#,
        )
        .await;

        let response = dataset_snapshots(
            State(test_state(pool)),
            ApiQuery::new(DatasetSnapshotsParams { limit: Some(10) }),
        )
        .await
        .unwrap()
        .0;

        assert_eq!(response.items.len(), 2);
        assert_eq!(response.items[0].id, 25);
        assert_eq!(response.items[0].station_version_count, 1);
        assert_eq!(response.items[0].change_counts.created, 1);
        assert_eq!(response.items[0].change_counts.removed, 1);
        assert_eq!(response.items[0].change_counts.total, 2);
        assert_eq!(response.items[1].id, 24);
        assert_eq!(response.items[1].change_counts.created, 1);
    }

    #[tokio::test]
    async fn dataset_changes_filters_by_snapshot_and_preserves_removed_station_context() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 24).await;
        insert_snapshot(&pool, 25).await;

        let old_version_id = insert_station(
            &pool,
            24,
            StationSeed::new("stn_n02_old", "旧駅", "旧線", "旧交通", 35.6000, 139.6000),
        )
        .await;
        let new_version_id = insert_station(
            &pool,
            25,
            StationSeed::new("stn_n02_new", "新駅", "新線", "新交通", 35.7000, 139.7000),
        )
        .await;

        insert_change_event(
            &pool,
            25,
            "stn_n02_new",
            "created",
            None,
            Some(new_version_id),
            r#"{"station_name":"新駅","line_name":"新線","operator_name":"新交通"}"#,
        )
        .await;
        insert_change_event(
            &pool,
            25,
            "stn_n02_old",
            "removed",
            Some(old_version_id),
            None,
            r#"{"station_name":"旧駅","line_name":"旧線","operator_name":"旧交通"}"#,
        )
        .await;

        let response = dataset_changes(
            State(test_state(pool)),
            ApiQuery::new(DatasetChangesParams {
                snapshot_id: Some(25),
                limit: Some(10),
            }),
        )
        .await
        .unwrap()
        .0;

        assert_eq!(response.snapshot_id, Some(25));
        assert_eq!(response.items.len(), 2);

        let removed = response
            .items
            .iter()
            .find(|item| item.change_kind == DatasetChangeKindDto::Removed)
            .unwrap();
        assert_eq!(removed.station_name.as_deref(), Some("旧駅"));
        assert_eq!(removed.line_name.as_deref(), Some("旧線"));
        assert_eq!(removed.after_version_id, None);
        assert_eq!(removed.detail.station_name.as_deref(), Some("旧駅"));

        let created = response
            .items
            .iter()
            .find(|item| item.change_kind == DatasetChangeKindDto::Created)
            .unwrap();
        assert_eq!(created.source_version.as_deref(), Some("N02-25"));
        assert_eq!(created.station_name.as_deref(), Some("新駅"));
    }

    #[tokio::test]
    async fn dataset_changes_returns_not_found_for_unknown_snapshot() {
        let response = dataset_changes(
            State(test_state(test_pool().await)),
            ApiQuery::new(DatasetChangesParams {
                snapshot_id: Some(999),
                limit: Some(10),
            }),
        )
        .await
        .unwrap_err()
        .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = json_body(response).await;
        assert_eq!(body["error"]["code"].as_str(), Some("not_found"));
    }

    #[tokio::test]
    async fn line_stations_keeps_enoshima_variants_separate() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 1).await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_enoshima_hiragana",
                "片瀬江ノ島",
                "江の島線",
                "小田急電鉄",
                35.3089,
                139.4807,
            ),
        )
        .await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_enoshima_katakana",
                "湘南江の島",
                "江ノ島線",
                "湘南モノレール",
                35.3112,
                139.4874,
            ),
        )
        .await;

        let state = test_state(pool);
        let response = line_stations(
            State(state.clone()),
            Path("江の島線".to_string()),
            ApiQuery::new(LineStationsParams {
                operator_name: None,
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response.items;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].station_name, "片瀬江ノ島");
        assert_eq!(items[0].line_name, "江の島線");

        let response = line_stations(
            State(state),
            Path("江ノ島線".to_string()),
            ApiQuery::new(LineStationsParams {
                operator_name: None,
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response.items;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].station_name, "湘南江の島");
        assert_eq!(items[0].line_name, "江ノ島線");
    }

    #[tokio::test]
    async fn line_stations_can_filter_same_line_by_operator() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 1).await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_central_jr",
                "新宿",
                "中央線",
                "東日本旅客鉄道",
                35.6900,
                139.7000,
            ),
        )
        .await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_central_subway",
                "中野坂上",
                "中央線",
                "東京地下鉄",
                35.6970,
                139.6820,
            ),
        )
        .await;

        let response = line_stations(
            State(test_state(pool)),
            Path("中央線".to_string()),
            ApiQuery::new(LineStationsParams {
                operator_name: Some("東京地下鉄".to_string()),
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response.items;

        assert_eq!(response.operator_name.as_deref(), Some("東京地下鉄"));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].operator_name, "東京地下鉄");
        assert_eq!(items[0].station_name, "中野坂上");
    }

    #[tokio::test]
    async fn search_stations_keeps_kana_variants_separate() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 1).await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_katase",
                "片瀬江ノ島",
                "江の島線",
                "小田急電鉄",
                35.3089,
                139.4807,
            ),
        )
        .await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_shonanenoshima",
                "湘南江の島",
                "江ノ島線",
                "湘南モノレール",
                35.3112,
                139.4874,
            ),
        )
        .await;

        let state = test_state(pool);
        let response = search_stations(
            State(state.clone()),
            ApiQuery::new(SearchParams {
                q: Some("江ノ島".to_string()),
                limit: Some(10),
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response.items;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].station_name, "片瀬江ノ島");
        assert_eq!(items[0].line_name, "江の島線");

        let response = search_stations(
            State(state),
            ApiQuery::new(SearchParams {
                q: Some("江の島".to_string()),
                limit: Some(10),
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response.items;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].station_name, "湘南江の島");
        assert_eq!(items[0].line_name, "江ノ島線");
    }

    #[tokio::test]
    async fn operator_stations_keeps_enoshima_variants_separate_and_sorted() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 1).await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_fujisawa",
                "藤沢",
                "江の島線",
                "小田急電鉄",
                35.3388,
                139.4876,
            ),
        )
        .await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_katase",
                "片瀬江ノ島",
                "江の島線",
                "小田急電鉄",
                35.3089,
                139.4807,
            ),
        )
        .await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_shonanenoshima",
                "湘南江の島",
                "江ノ島線",
                "小田急電鉄",
                35.3112,
                139.4874,
            ),
        )
        .await;

        let response = operator_stations(State(test_state(pool)), Path("小田急電鉄".to_string()))
            .await
            .unwrap()
            .0;
        let items = response.items;

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].line_name, "江の島線");
        assert_eq!(items[0].station_name, "片瀬江ノ島");
        assert_eq!(items[1].line_name, "江の島線");
        assert_eq!(items[1].station_name, "藤沢");
        assert_eq!(items[2].line_name, "江ノ島線");
        assert_eq!(items[2].station_name, "湘南江の島");
    }

    #[tokio::test]
    async fn line_catalog_keeps_enoshima_variants_separate() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 1).await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_enoshima_hiragana",
                "片瀬江ノ島",
                "江の島線",
                "小田急電鉄",
                35.3089,
                139.4807,
            ),
        )
        .await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_enoshima_katakana",
                "湘南江の島",
                "江ノ島線",
                "湘南モノレール",
                35.3112,
                139.4874,
            ),
        )
        .await;

        let response = line_catalog(
            State(test_state(pool)),
            ApiQuery::new(LineCatalogParams {
                q: Some("江".to_string()),
                limit: Some(10),
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response.items;

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| {
            item.line_name == "江の島線"
                && item.operator_name == "小田急電鉄"
                && item.station_count == 1
        }));
        assert!(items.iter().any(|item| {
            item.line_name == "江ノ島線"
                && item.operator_name == "湘南モノレール"
                && item.station_count == 1
        }));
    }

    #[tokio::test]
    async fn line_catalog_separates_same_line_by_operator() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 1).await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_central_jr",
                "新宿",
                "中央線",
                "東日本旅客鉄道",
                35.6900,
                139.7000,
            ),
        )
        .await;
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_central_subway",
                "中野坂上",
                "中央線",
                "東京地下鉄",
                35.6970,
                139.6820,
            ),
        )
        .await;

        let response = line_catalog(
            State(test_state(pool)),
            ApiQuery::new(LineCatalogParams {
                q: Some("中央".to_string()),
                limit: Some(10),
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response.items;

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| {
            item.line_name == "中央線" && item.operator_name == "東日本旅客鉄道"
        }));
        assert!(items.iter().any(|item| {
            item.line_name == "中央線" && item.operator_name == "東京地下鉄"
        }));
    }

    #[tokio::test]
    async fn dataset_status_scopes_counts_to_n02_rows_and_uses_latest_snapshot_metadata() {
        let pool = test_pool().await;
        insert_snapshot(&pool, 24).await;
        insert_snapshot(&pool, 25).await;
        insert_station(
            &pool,
            24,
            StationSeed::new(
                "stn_n02_shinjuku",
                "新宿",
                "中央線",
                "東日本旅客鉄道",
                35.6900,
                139.7000,
            ),
        )
        .await;
        insert_station(
            &pool,
            25,
            StationSeed::new(
                "stn_n02_shibuya",
                "渋谷",
                "山手線",
                "東日本旅客鉄道",
                35.6580,
                139.7016,
            ),
        )
        .await;
        insert_station(
            &pool,
            25,
            StationSeed::new(
                "stn_other_network",
                "ノイズ駅",
                "ノイズ線",
                "ノイズ交通",
                35.6000,
                139.6000,
            ),
        )
        .await;

        let response = dataset_status(State(test_state(pool))).await.unwrap().0;

        assert_eq!(response.active_station_count, 2);
        assert_eq!(response.distinct_station_name_count, 2);
        assert_eq!(response.distinct_line_count, 2);
        assert_eq!(response.active_version_snapshot_count, 2);
        assert_eq!(
            response
                .active_snapshot
                .as_ref()
                .map(|snapshot| snapshot.id),
            Some(25)
        );
        assert_eq!(
            response
                .active_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.source_version.as_deref()),
            Some("N02-25")
        );
    }

    #[tokio::test]
    async fn dataset_status_surfaces_snapshot_decode_failures() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO source_snapshots (id, source_name, source_kind, source_version, source_url, source_sha256)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(1_i64)
        .bind(N02_SOURCE_NAME)
        .bind("geojson_zip_entry")
        .bind("N02-25")
        .bind(vec![0xff_u8])
        .bind("sha-1")
        .execute(&pool)
        .await
        .unwrap();

        let response = dataset_status(State(test_state(pool)))
            .await
            .unwrap_err()
            .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn dataset_status_treats_uppercase_http_snapshot_urls_as_remote() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO source_snapshots (id, source_name, source_kind, source_version, source_url, source_sha256)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(1_i64)
        .bind(N02_SOURCE_NAME)
        .bind("geojson_zip_entry")
        .bind("N02-25")
        .bind("HTTPS://example.com/N02-25_GML.zip")
        .bind("sha-1")
        .execute(&pool)
        .await
        .unwrap();
        insert_station(
            &pool,
            1,
            StationSeed::new(
                "stn_n02_shinjuku",
                "新宿",
                "中央線",
                "東日本旅客鉄道",
                35.6900,
                139.7000,
            ),
        )
        .await;

        let response = dataset_status(State(test_state(pool))).await.unwrap().0;

        assert!(!response.source_is_local);
    }

    fn get(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    async fn json_body(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn text_body(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn test_state(pool: AnyPool) -> AppState {
        AppState {
            config: AppConfig {
                service_name: "station-api-test".to_string(),
                bind_addr: "127.0.0.1:0".to_string(),
                database_type: DatabaseType::Sqlite,
                database_url: "sqlite::memory:".to_string(),
                job_lock_dir: "tmp/locks".to_string(),
                redis_url: None,
                ready_require_cache: false,
                update_interval_seconds: 0,
                source_snapshot_url: None,
                allow_local_source_snapshot: false,
                temp_asset_dir: "tmp".to_string(),
                ingest_write_chunk_size: 1000,
                ingest_close_chunk_size: 1000,
            },
            dialect: SqlDialect::Sqlite,
            pool,
        }
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

    async fn insert_snapshot(pool: &AnyPool, id: i64) {
        sqlx::query(
            "INSERT INTO source_snapshots (id, source_name, source_kind, source_version, source_url, source_sha256)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(N02_SOURCE_NAME)
        .bind("geojson_zip_entry")
        .bind(format!("N02-{id}"))
        .bind(format!("https://example.com/N02-{id}_GML.zip"))
        .bind(format!("sha-{id}"))
        .execute(pool)
        .await
        .unwrap();
    }

    struct StationSeed<'a> {
        station_uid: &'a str,
        station_name: &'a str,
        line_name: &'a str,
        operator_name: &'a str,
        latitude: f64,
        longitude: f64,
    }

    impl<'a> StationSeed<'a> {
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

    async fn insert_station(pool: &AnyPool, snapshot_id: i64, station: StationSeed<'_>) -> i64 {
        sqlx::query(
            "INSERT INTO station_identities (station_uid, canonical_name)
             VALUES (?, ?)",
        )
        .bind(station.station_uid)
        .bind(station.station_name)
        .execute(pool)
        .await
        .unwrap();

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
            "{}:{snapshot_id}:{}:{}",
            station.station_uid, station.line_name, station.station_name
        ))
        .execute(pool)
        .await
        .unwrap();

        sqlx::query("SELECT id FROM station_versions WHERE station_uid = ? AND snapshot_id = ?")
            .bind(station.station_uid)
            .bind(snapshot_id)
            .fetch_one(pool)
            .await
            .unwrap()
            .try_get::<i64, _>("id")
            .unwrap()
    }

    async fn insert_change_event(
        pool: &AnyPool,
        snapshot_id: i64,
        station_uid: &str,
        change_kind: &str,
        before_version_id: Option<i64>,
        after_version_id: Option<i64>,
        detail_json: &str,
    ) {
        sqlx::query(
            "INSERT INTO station_change_events (
               snapshot_id,
               station_uid,
               change_kind,
               before_version_id,
               after_version_id,
               detail_json
             ) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(snapshot_id)
        .bind(station_uid)
        .bind(change_kind)
        .bind(before_version_id)
        .bind(after_version_id)
        .bind(detail_json)
        .execute(pool)
        .await
        .unwrap();
    }
}
