use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchParams {
    #[param(example = "新宿")]
    pub q: Option<String>,
    #[param(example = 10, minimum = 1, maximum = 100)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LineCatalogParams {
    #[param(example = "中央")]
    pub q: Option<String>,
    #[param(example = 60, minimum = 1, maximum = 1000)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct NearbyParams {
    #[param(example = 35.6812)]
    pub lat: f64,
    #[param(example = 139.7671)]
    pub lng: f64,
    #[param(example = 10, minimum = 1, maximum = 100)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LineStationsParams {
    #[param(example = "東日本旅客鉄道")]
    pub operator_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DatasetSnapshotsParams {
    /// Maximum number of recent N02 source snapshots to return.
    #[param(example = 20, minimum = 1, maximum = 200)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DatasetChangesParams {
    /// Optional source snapshot id to filter change events.
    #[param(example = 25)]
    pub snapshot_id: Option<i64>,
    /// Maximum number of recent N02 change events to return.
    #[param(example = 20, minimum = 1, maximum = 200)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    InvalidRequest,
    NotFound,
    InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "code": "internal_error",
    "message": "internal server error"
}))]
pub struct ApiErrorDetailDto {
    /// Stable machine-readable error code.
    pub code: ApiErrorCode,
    /// Human-readable error message.
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "error": {
        "code": "internal_error",
        "message": "internal server error"
    }
}))]
pub struct ApiErrorResponseDto {
    pub error: ApiErrorDetailDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "status": "ok",
    "service": "station-api"
}))]
pub struct HealthResponseDto {
    pub status: String,
    pub service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "status": "ready",
    "database_type": "postgres",
    "cache": "disabled"
}))]
pub struct ReadinessResponseDto {
    pub status: String,
    pub database_type: String,
    pub cache: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "id": 25,
    "source_version": "N02-25",
    "source_url": "https://example.com/N02-25_GML.zip"
}))]
pub struct DatasetSnapshotRefDto {
    pub id: i64,
    pub source_version: Option<String>,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[schema(example = json!({
    "status": "ready",
    "looks_like_full_dataset": true,
    "source_is_local": false,
    "active_station_count": 10155,
    "distinct_station_name_count": 9102,
    "distinct_line_count": 623,
    "active_version_snapshot_count": 2,
    "active_snapshot": {
        "id": 25,
        "source_version": "N02-25",
        "source_url": "https://example.com/N02-25_GML.zip"
    }
}))]
pub struct DatasetStatusResponseDto {
    pub status: String,
    pub looks_like_full_dataset: bool,
    pub source_is_local: bool,
    pub active_station_count: i64,
    pub distinct_station_name_count: i64,
    pub distinct_line_count: i64,
    pub active_version_snapshot_count: i64,
    pub active_snapshot: Option<DatasetSnapshotRefDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[schema(example = json!({
    "station_uid": "stn_jp_example_shinjuku",
    "station_name": "新宿",
    "line_name": "山手線",
    "operator_name": "東日本旅客鉄道",
    "latitude": 35.6909,
    "longitude": 139.7003,
    "status": "active"
}))]
pub struct StationSummaryDto {
    pub station_uid: String,
    pub station_name: String,
    pub line_name: String,
    pub operator_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[schema(example = json!({
    "items": [{
        "station_uid": "stn_jp_example_shinjuku",
        "station_name": "新宿",
        "line_name": "山手線",
        "operator_name": "東日本旅客鉄道",
        "latitude": 35.6909,
        "longitude": 139.7003,
        "status": "active"
    }],
    "limit": 10,
    "query": "新宿"
}))]
pub struct StationSearchResponseDto {
    pub items: Vec<StationSummaryDto>,
    pub limit: i64,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[schema(example = json!({
    "lat": 35.6895,
    "lng": 139.6917
}))]
pub struct NearbyStationsQueryDto {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[schema(example = json!({
    "items": [{
        "station_uid": "stn_jp_example_shinjuku",
        "station_name": "新宿",
        "line_name": "山手線",
        "operator_name": "東日本旅客鉄道",
        "latitude": 35.6909,
        "longitude": 139.7003,
        "status": "active"
    }],
    "limit": 10,
    "query": {
        "lat": 35.6895,
        "lng": 139.6917
    }
}))]
pub struct NearbyStationsResponseDto {
    pub items: Vec<StationSummaryDto>,
    pub limit: i64,
    pub query: NearbyStationsQueryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "line_name": "中央線",
    "operator_name": "東日本旅客鉄道",
    "station_count": 24
}))]
pub struct LineCatalogItemDto {
    pub line_name: String,
    pub operator_name: String,
    pub station_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "items": [{
        "line_name": "中央線",
        "operator_name": "東日本旅客鉄道",
        "station_count": 24
    }],
    "limit": 20,
    "query": "中央"
}))]
pub struct LineCatalogResponseDto {
    pub items: Vec<LineCatalogItemDto>,
    pub limit: i64,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[schema(example = json!({
    "line_name": "中央線",
    "operator_name": "東日本旅客鉄道",
    "items": [{
        "station_uid": "stn_jp_example_shinjuku",
        "station_name": "新宿",
        "line_name": "中央線",
        "operator_name": "東日本旅客鉄道",
        "latitude": 35.6909,
        "longitude": 139.7003,
        "status": "active"
    }]
}))]
pub struct LineStationsResponseDto {
    pub line_name: String,
    pub operator_name: Option<String>,
    pub items: Vec<StationSummaryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[schema(example = json!({
    "operator_name": "東日本旅客鉄道",
    "items": [{
        "station_uid": "stn_jp_example_shinjuku",
        "station_name": "新宿",
        "line_name": "山手線",
        "operator_name": "東日本旅客鉄道",
        "latitude": 35.6909,
        "longitude": 139.7003,
        "status": "active"
    }]
}))]
pub struct OperatorStationsResponseDto {
    pub operator_name: String,
    pub items: Vec<StationSummaryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "created": 12,
    "updated": 4,
    "removed": 1,
    "total": 17
}))]
pub struct DatasetSnapshotChangeCountsDto {
    /// Number of station identities created by this source snapshot.
    pub created: i64,
    /// Number of station identities updated by this source snapshot.
    pub updated: i64,
    /// Number of station identities removed by this source snapshot.
    pub removed: i64,
    /// Sum of created, updated, and removed change events.
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "id": 25,
    "source_name": "ksj_n02_station",
    "source_kind": "geojson_zip_entry",
    "source_version": "N02-25",
    "source_url": "https://example.com/N02-25_GML.zip",
    "source_sha256": "84d675d10bfe01b7fdcbe97cf9221c0b5054d5833cf9a339b37e8b82ac3bd5aa",
    "downloaded_at": "2026-04-20 12:34:56",
    "station_version_count": 10155,
    "change_counts": {
        "created": 12,
        "updated": 4,
        "removed": 1,
        "total": 17
    }
}))]
pub struct DatasetSnapshotDto {
    /// Internal source snapshot id. Use this as `snapshot_id` for `/v1/dataset/changes`.
    pub id: i64,
    /// Canonical source name. N02 station snapshots use `ksj_n02_station`.
    pub source_name: String,
    /// Stored source format for the snapshot.
    pub source_kind: String,
    /// MLIT source version when it can be derived from the source package.
    pub source_version: Option<String>,
    /// Original source URL or local fixture URL used for ingest.
    pub source_url: String,
    /// SHA-256 digest of the ingested source package.
    pub source_sha256: String,
    /// Snapshot download or load timestamp string emitted by the active database dialect.
    pub downloaded_at: String,
    /// Number of N02 station versions attached to this snapshot.
    pub station_version_count: i64,
    /// Change event counts scoped to N02 station identities.
    pub change_counts: DatasetSnapshotChangeCountsDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "items": [{
        "id": 25,
        "source_name": "ksj_n02_station",
        "source_kind": "geojson_zip_entry",
        "source_version": "N02-25",
        "source_url": "https://example.com/N02-25_GML.zip",
        "source_sha256": "84d675d10bfe01b7fdcbe97cf9221c0b5054d5833cf9a339b37e8b82ac3bd5aa",
        "downloaded_at": "2026-04-20 12:34:56",
        "station_version_count": 10155,
        "change_counts": {
            "created": 12,
            "updated": 4,
            "removed": 1,
            "total": 17
        }
    }],
    "limit": 20
}))]
pub struct DatasetSnapshotsResponseDto {
    /// Source snapshots ordered newest first.
    pub items: Vec<DatasetSnapshotDto>,
    /// Normalized response limit.
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DatasetChangeKindDto {
    /// A station identity appeared for the first time in this snapshot.
    Created,
    /// A station identity existed before and one or more tracked fields changed.
    Updated,
    /// A station identity from an earlier snapshot no longer appears in this snapshot.
    Removed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "station_name": "新宿",
    "line_name": "中央線",
    "operator_name": "東日本旅客鉄道",
    "source_station_code": "003700",
    "source_group_code": "003700",
    "status": "active"
}))]
pub struct DatasetChangeVersionRefDto {
    /// Station name from the referenced station version.
    pub station_name: Option<String>,
    /// Line name from the referenced station version.
    pub line_name: Option<String>,
    /// Operator name from the referenced station version.
    pub operator_name: Option<String>,
    /// MLIT station code when available.
    pub source_station_code: Option<String>,
    /// MLIT group code when available.
    pub source_group_code: Option<String>,
    /// Status from the referenced station version; current ingest persists this as `active`.
    /// Removals are represented by the surrounding dataset change metadata, not by this field.
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "changed_fields": ["line_name"],
    "before": {
        "station_name": "新宿",
        "line_name": "京王線",
        "operator_name": "京王電鉄",
        "source_station_code": "003700",
        "source_group_code": "003700",
        "status": "active"
    },
    "after": {
        "station_name": "新宿",
        "line_name": "京王新線",
        "operator_name": "京王電鉄",
        "source_station_code": "003700",
        "source_group_code": "003700",
        "status": "active"
    }
}))]
pub struct DatasetChangeDetailDto {
    /// Field names that changed for an `updated` event.
    #[serde(default)]
    pub changed_fields: Vec<String>,
    /// Previous station version context for `updated` events when available.
    #[schema(inline)]
    pub before: Option<DatasetChangeVersionRefDto>,
    /// New station version context for `updated` events when available.
    #[schema(inline)]
    pub after: Option<DatasetChangeVersionRefDto>,
    /// Flat station name context used by `created` and `removed` events.
    pub station_name: Option<String>,
    /// Flat line name context used by `created` and `removed` events.
    pub line_name: Option<String>,
    /// Flat operator name context used by `created` and `removed` events.
    pub operator_name: Option<String>,
    /// Flat MLIT station code context used by `created` and `removed` events.
    pub source_station_code: Option<String>,
    /// Flat MLIT group code context used by `created` and `removed` events.
    pub source_group_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "id": 42,
    "snapshot_id": 25,
    "source_version": "N02-25",
    "station_uid": "stn_n02_003700_49ed2e7fc9a4cd46",
    "change_kind": "updated",
    "station_name": "新宿",
    "line_name": "京王新線",
    "operator_name": "京王電鉄",
    "before_version_id": 10,
    "after_version_id": 11,
    "detail": {
        "changed_fields": ["line_name"],
        "before": {
            "station_name": "新宿",
            "line_name": "京王線",
            "operator_name": "京王電鉄",
            "source_station_code": "003700",
            "source_group_code": "003700",
            "status": "active"
        },
        "after": {
            "station_name": "新宿",
            "line_name": "京王新線",
            "operator_name": "京王電鉄",
            "source_station_code": "003700",
            "source_group_code": "003700",
            "status": "active"
        }
    },
    "created_at": "2026-04-20 12:34:56"
}))]
pub struct DatasetChangeEventDto {
    /// Internal change event id, ordered newest first by this endpoint.
    pub id: i64,
    /// Source snapshot id that produced this change.
    pub snapshot_id: i64,
    /// MLIT source version associated with the source snapshot when available.
    pub source_version: Option<String>,
    /// Stable station identity used by this dataset.
    pub station_uid: String,
    /// Type of change recorded for the station identity.
    pub change_kind: DatasetChangeKindDto,
    /// Best available station name context from the before or after version.
    pub station_name: Option<String>,
    /// Best available line name context from the before or after version.
    pub line_name: Option<String>,
    /// Best available operator name context from the before or after version.
    pub operator_name: Option<String>,
    /// Previous station version id for `updated` and `removed` events.
    pub before_version_id: Option<i64>,
    /// New station version id for `created` and `updated` events.
    pub after_version_id: Option<i64>,
    /// Structured before/after context for consumers that need field-level diffs.
    pub detail: DatasetChangeDetailDto,
    /// Change event creation timestamp string emitted by the active database dialect.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "items": [{
        "id": 42,
        "snapshot_id": 25,
        "source_version": "N02-25",
        "station_uid": "stn_n02_003700_49ed2e7fc9a4cd46",
        "change_kind": "updated",
        "station_name": "新宿",
        "line_name": "京王新線",
        "operator_name": "京王電鉄",
        "before_version_id": 10,
        "after_version_id": 11,
        "detail": {
            "changed_fields": ["line_name"],
            "before": {
                "station_name": "新宿",
                "line_name": "京王線",
                "operator_name": "京王電鉄",
                "source_station_code": "003700",
                "source_group_code": "003700",
                "status": "active"
            },
            "after": {
                "station_name": "新宿",
                "line_name": "京王新線",
                "operator_name": "京王電鉄",
                "source_station_code": "003700",
                "source_group_code": "003700",
                "status": "active"
            }
        },
        "created_at": "2026-04-20 12:34:56"
    }],
    "limit": 20,
    "snapshot_id": 25
}))]
pub struct DatasetChangesResponseDto {
    /// Change events ordered newest first.
    pub items: Vec<DatasetChangeEventDto>,
    /// Normalized response limit.
    pub limit: i64,
    /// Echoes the requested snapshot filter, or null when no filter was used.
    pub snapshot_id: Option<i64>,
}
