use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{AnyPool, Row};
use station_shared::{
    config::AppConfig,
    db::{
        connect_any_pool, decode_optional_string, decode_required_string, distinct_text_count_sql,
        integer_aggregate_sql, SqlDialect,
    },
    model::{HealthResponse, ReadyResponse, StationSummary},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

const FULL_DATASET_MIN_STATION_COUNT: i64 = 10_000;
const N02_SOURCE_NAME: &str = "ksj_n02_station";
const N02_STATION_UID_PREFIX: &str = "stn_n02_";

#[derive(Clone)]
struct AppState {
    config: AppConfig,
    dialect: SqlDialect,
    pool: AnyPool,
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: Option<String>,
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct NearbyParams {
    lat: f64,
    lng: f64,
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct LineStationsParams {
    operator_name: Option<String>,
}

#[tokio::main]
async fn main() {
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

    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/v1/dataset/status", get(dataset_status))
        .route("/v1/stations/search", get(search_stations))
        .route("/v1/stations/nearby", get(nearby_stations))
        .route("/v1/lines/catalog", get(line_catalog))
        .route("/v1/lines/{line_name}/stations", get(line_stations))
        .route(
            "/v1/operators/{operator_name}/stations",
            get(operator_stations),
        )
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    info!("starting {} on {}", config.service_name, config.bind_addr);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .expect("bind failed");

    axum::serve(listener, app).await.expect("server failed");
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        service: state.config.service_name,
    })
}

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
        Ok(_) => Json(ReadyResponse {
            status: "ready",
            database_type: state.config.database_type.to_string(),
            cache,
        })
        .into_response(),
        Err(err) => {
            error!(error = %err, "database readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "status": "not_ready",
                    "database_type": state.config.database_type.to_string(),
                    "cache": cache,
                })),
            )
                .into_response()
        }
    }
}

async fn search_stations(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let query = params.q.unwrap_or_default().trim().to_string();
    let limit = normalized_limit(params.limit);

    if query.is_empty() {
        return Ok(Json(json!({
            "items": [],
            "limit": limit,
            "query": query,
        })));
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

    Ok(Json(json!({
        "items": items,
        "limit": limit,
        "query": query,
    })))
}

async fn dataset_status(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let n02_where_clause = format!(
        "WHERE {}",
        station_uid_prefix_scope_sql(state.dialect, "station_uid")
    );
    let counts_sql = format!(
        "SELECT
           {} AS active_station_count,
           {} AS distinct_station_name_count,
           {} AS distinct_line_count,
           {} AS active_version_snapshot_count
         FROM stations_latest
         {n02_where_clause}",
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
    );
    let counts = sqlx::query(&state.dialect.statement(&counts_sql))
        .bind(station_uid_prefix_scope_arg(
            state.dialect,
            N02_STATION_UID_PREFIX,
        ))
        .fetch_one(&state.pool)
        .await
        .map_err(internal_error)?;

    let active_station_count = counts
        .try_get::<i64, _>("active_station_count")
        .map_err(internal_error)?;
    let distinct_station_name_count = counts
        .try_get::<i64, _>("distinct_station_name_count")
        .map_err(internal_error)?;
    let distinct_line_count = counts
        .try_get::<i64, _>("distinct_line_count")
        .map_err(internal_error)?;
    let active_version_snapshot_count = counts
        .try_get::<i64, _>("active_version_snapshot_count")
        .map_err(internal_error)?;

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
        let source_version = decode_optional_string(&row, "source_version")
            .map_err(|err| sqlx::Error::Decode(err.into()))?;
        let source_url = decode_required_string(&row, "source_url")
            .map_err(|err| sqlx::Error::Decode(err.into()))?;

        Ok::<Value, sqlx::Error>(json!({
            "id": id,
            "source_version": source_version,
            "source_url": source_url,
        }))
    });
    let active_snapshot = active_snapshot.transpose().map_err(internal_error)?;

    let source_url = active_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("source_url"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let source_is_local = !(source_url.is_empty() || is_remote_http_url(source_url));
    let looks_like_full_dataset = active_station_count >= FULL_DATASET_MIN_STATION_COUNT;

    Ok(Json(json!({
        "status": if looks_like_full_dataset { "ready" } else { "needs_ingest" },
        "looks_like_full_dataset": looks_like_full_dataset,
        "source_is_local": source_is_local,
        "active_station_count": active_station_count,
        "distinct_station_name_count": distinct_station_name_count,
        "distinct_line_count": distinct_line_count,
        "active_version_snapshot_count": active_version_snapshot_count,
        "active_snapshot": active_snapshot,
    })))
}

fn is_remote_http_url(url: &str) -> bool {
    let Some((scheme, _)) = url.split_once("://") else {
        return false;
    };

    scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http")
}

async fn nearby_stations(
    State(state): State<AppState>,
    Query(params): Query<NearbyParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
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

    Ok(Json(json!({
        "items": items,
        "limit": limit,
        "query": {
            "lat": params.lat,
            "lng": params.lng,
        }
    })))
}

async fn line_catalog(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
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
            Ok(json!({
                "line_name": decode_required_string(&row, "line_name")
                    .map_err(|err| sqlx::Error::Decode(err.into()))?,
                "operator_name": decode_required_string(&row, "operator_name")
                    .map_err(|err| sqlx::Error::Decode(err.into()))?,
                "station_count": row.try_get::<i64, _>("station_count")?,
            }))
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(internal_error)?;

    Ok(Json(json!({
        "items": items,
        "limit": limit,
        "query": query,
    })))
}

async fn line_stations(
    State(state): State<AppState>,
    Path(line_name): Path<String>,
    Query(params): Query<LineStationsParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let line_name_match = text_equals_sql(state.dialect, "line_name");
    let operator_name_match = text_equals_sql(state.dialect, "operator_name");
    let operator_name_order = text_order_sql(state.dialect, "operator_name");
    let station_name_order = text_order_sql(state.dialect, "station_name");
    let operator_name = params
        .operator_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
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
    let rows = if let Some(operator_name) = operator_name {
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

    Ok(Json(json!({
        "line_name": line_name,
        "operator_name": operator_name,
        "items": items,
    })))
}

async fn operator_stations(
    State(state): State<AppState>,
    Path(operator_name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
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

    Ok(Json(json!({
        "operator_name": operator_name,
        "items": items,
    })))
}

fn normalized_limit(limit: Option<u32>) -> i64 {
    i64::from(limit.unwrap_or(10).clamp(1, 100))
}

fn normalized_catalog_limit(limit: Option<u32>) -> i64 {
    i64::from(limit.unwrap_or(60).clamp(1, 1000))
}

fn row_to_station_summary(row: sqlx::any::AnyRow) -> Result<StationSummary, sqlx::Error> {
    Ok(StationSummary {
        station_uid: row.try_get("station_uid")?,
        station_name: decode_required_string(&row, "station_name")
            .map_err(|err| sqlx::Error::Decode(err.into()))?,
        line_name: decode_required_string(&row, "line_name")
            .map_err(|err| sqlx::Error::Decode(err.into()))?,
        operator_name: decode_required_string(&row, "operator_name")
            .map_err(|err| sqlx::Error::Decode(err.into()))?,
        latitude: row.try_get("latitude")?,
        longitude: row.try_get("longitude")?,
        status: decode_required_string(&row, "status")
            .map_err(|err| sqlx::Error::Decode(err.into()))?,
    })
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    error!(error = %error, "API request failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "error": "internal_server_error",
        })),
    )
}

#[cfg(test)]
fn nullable_integer_aggregate_sql(dialect: SqlDialect, expr: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("CAST({expr} AS SIGNED)"),
        SqlDialect::Postgres | SqlDialect::Sqlite => format!("CAST({expr} AS BIGINT)"),
    }
}

fn station_uid_prefix_scope_sql(dialect: SqlDialect, column: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("{column} COLLATE utf8mb4_bin LIKE ? ESCAPE '\\\\'"),
        SqlDialect::Postgres | SqlDialect::Sqlite => {
            format!("substr({column}, 1, {}) = ?", N02_STATION_UID_PREFIX.len())
        }
    }
}

fn station_uid_prefix_scope_arg(dialect: SqlDialect, prefix: &str) -> String {
    match dialect {
        SqlDialect::Mysql => like_prefix_pattern(prefix),
        SqlDialect::Postgres | SqlDialect::Sqlite => prefix.to_string(),
    }
}

fn like_prefix_pattern(prefix: &str) -> String {
    let escaped = prefix
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("{escaped}%")
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
        extract::{Path, Query, State},
        http::StatusCode,
    };
    use sqlx::{any::AnyPoolOptions, AnyPool};
    use station_shared::{
        config::{AppConfig, DatabaseType},
        db::{distinct_text_count_sql, ensure_sqlx_drivers, integer_aggregate_sql, SqlDialect},
    };

    use super::{
        dataset_status, like_prefix_pattern, line_catalog, line_stations,
        nullable_integer_aggregate_sql, operator_stations, search_stations,
        station_uid_prefix_scope_arg, station_uid_prefix_scope_sql, text_equals_sql,
        text_group_sql, text_like_sql, text_order_sql, AppState, LineStationsParams, SearchParams,
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
            station_uid_prefix_scope_sql(SqlDialect::Mysql, "station_uid"),
            "station_uid COLLATE utf8mb4_bin LIKE ? ESCAPE '\\\\'"
        );
        assert_eq!(
            station_uid_prefix_scope_arg(SqlDialect::Mysql, N02_STATION_UID_PREFIX),
            "stn\\_n02\\_%"
        );
    }

    #[test]
    fn sqlite_station_uid_prefix_scope_stays_substr_exact() {
        assert_eq!(
            station_uid_prefix_scope_sql(SqlDialect::Sqlite, "station_uid"),
            "substr(station_uid, 1, 8) = ?"
        );
        assert_eq!(
            station_uid_prefix_scope_arg(SqlDialect::Sqlite, N02_STATION_UID_PREFIX),
            N02_STATION_UID_PREFIX
        );
        assert_eq!(like_prefix_pattern("stn_n02_"), "stn\\_n02\\_%");
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
            Query(LineStationsParams {
                operator_name: None,
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response["items"].as_array().unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["station_name"].as_str(), Some("片瀬江ノ島"));
        assert_eq!(items[0]["line_name"].as_str(), Some("江の島線"));

        let response = line_stations(
            State(state),
            Path("江ノ島線".to_string()),
            Query(LineStationsParams {
                operator_name: None,
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response["items"].as_array().unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["station_name"].as_str(), Some("湘南江の島"));
        assert_eq!(items[0]["line_name"].as_str(), Some("江ノ島線"));
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
            Query(LineStationsParams {
                operator_name: Some("東京地下鉄".to_string()),
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response["items"].as_array().unwrap();

        assert_eq!(response["operator_name"].as_str(), Some("東京地下鉄"));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["operator_name"].as_str(), Some("東京地下鉄"));
        assert_eq!(items[0]["station_name"].as_str(), Some("中野坂上"));
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
            Query(SearchParams {
                q: Some("江ノ島".to_string()),
                limit: Some(10),
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response["items"].as_array().unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["station_name"].as_str(), Some("片瀬江ノ島"));
        assert_eq!(items[0]["line_name"].as_str(), Some("江の島線"));

        let response = search_stations(
            State(state),
            Query(SearchParams {
                q: Some("江の島".to_string()),
                limit: Some(10),
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response["items"].as_array().unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["station_name"].as_str(), Some("湘南江の島"));
        assert_eq!(items[0]["line_name"].as_str(), Some("江ノ島線"));
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
        let items = response["items"].as_array().unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["line_name"].as_str(), Some("江の島線"));
        assert_eq!(items[0]["station_name"].as_str(), Some("片瀬江ノ島"));
        assert_eq!(items[1]["line_name"].as_str(), Some("江の島線"));
        assert_eq!(items[1]["station_name"].as_str(), Some("藤沢"));
        assert_eq!(items[2]["line_name"].as_str(), Some("江ノ島線"));
        assert_eq!(items[2]["station_name"].as_str(), Some("湘南江の島"));
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
            Query(SearchParams {
                q: Some("江".to_string()),
                limit: Some(10),
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response["items"].as_array().unwrap();

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| {
            item["line_name"].as_str() == Some("江の島線")
                && item["operator_name"].as_str() == Some("小田急電鉄")
                && item["station_count"].as_i64() == Some(1)
        }));
        assert!(items.iter().any(|item| {
            item["line_name"].as_str() == Some("江ノ島線")
                && item["operator_name"].as_str() == Some("湘南モノレール")
                && item["station_count"].as_i64() == Some(1)
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
            Query(SearchParams {
                q: Some("中央".to_string()),
                limit: Some(10),
            }),
        )
        .await
        .unwrap()
        .0;
        let items = response["items"].as_array().unwrap();

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| {
            item["line_name"].as_str() == Some("中央線")
                && item["operator_name"].as_str() == Some("東日本旅客鉄道")
        }));
        assert!(items.iter().any(|item| {
            item["line_name"].as_str() == Some("中央線")
                && item["operator_name"].as_str() == Some("東京地下鉄")
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

        assert_eq!(response["active_station_count"].as_i64(), Some(2));
        assert_eq!(response["distinct_station_name_count"].as_i64(), Some(2));
        assert_eq!(response["distinct_line_count"].as_i64(), Some(2));
        assert_eq!(response["active_version_snapshot_count"].as_i64(), Some(2));
        assert_eq!(response["active_snapshot"]["id"].as_i64(), Some(25));
        assert_eq!(
            response["active_snapshot"]["source_version"].as_str(),
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

        let error = dataset_status(State(test_state(pool))).await.unwrap_err();

        assert_eq!(error.0, StatusCode::INTERNAL_SERVER_ERROR);
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

        assert_eq!(response["source_is_local"].as_bool(), Some(false));
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

    async fn insert_station(pool: &AnyPool, snapshot_id: i64, station: StationSeed<'_>) {
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
    }
}
