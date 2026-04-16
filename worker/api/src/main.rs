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
    db::{connect_any_pool, SqlDialect},
    model::{HealthResponse, ReadyResponse, StationSummary},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

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
        .route("/v1/stations/search", get(search_stations))
        .route("/v1/stations/nearby", get(nearby_stations))
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
         WHERE station_name LIKE ?
         ORDER BY
           CASE
             WHEN station_name = ? THEN 0
             WHEN station_name LIKE ? THEN 1
             ELSE 2
           END,
           operator_name,
           line_name,
           station_name
         LIMIT ?",
    ))
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

async fn line_stations(
    State(state): State<AppState>,
    Path(line_name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
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
         WHERE line_name = ?
         ORDER BY operator_name, station_name",
    ))
    .bind(&line_name)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    let items = rows
        .into_iter()
        .map(row_to_station_summary)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;

    Ok(Json(json!({
        "line_name": line_name,
        "items": items,
    })))
}

async fn operator_stations(
    State(state): State<AppState>,
    Path(operator_name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
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
         WHERE operator_name = ?
         ORDER BY line_name, station_name",
    ))
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

fn row_to_station_summary(row: sqlx::any::AnyRow) -> Result<StationSummary, sqlx::Error> {
    Ok(StationSummary {
        station_uid: row.try_get("station_uid")?,
        station_name: row.try_get("station_name")?,
        line_name: row.try_get("line_name")?,
        operator_name: row.try_get("operator_name")?,
        latitude: row.try_get("latitude")?,
        longitude: row.try_get("longitude")?,
        status: row.try_get("status")?,
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
