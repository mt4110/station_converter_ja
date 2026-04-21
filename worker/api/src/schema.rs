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
    #[param(example = 20, minimum = 1, maximum = 200)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DatasetChangesParams {
    #[param(example = 25)]
    pub snapshot_id: Option<i64>,
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
    pub code: ApiErrorCode,
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
    pub created: i64,
    pub updated: i64,
    pub removed: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[schema(example = json!({
    "id": 25,
    "source_name": "ksj_n02_station",
    "source_kind": "geojson_zip_entry",
    "source_version": "N02-25",
    "source_url": "https://example.com/N02-25_GML.zip",
    "source_sha256": "sha-25",
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
    pub id: i64,
    pub source_name: String,
    pub source_kind: String,
    pub source_version: Option<String>,
    pub source_url: String,
    pub source_sha256: String,
    pub downloaded_at: String,
    pub station_version_count: i64,
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
        "source_sha256": "sha-25",
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
    pub items: Vec<DatasetSnapshotDto>,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DatasetChangeKindDto {
    Created,
    Updated,
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
    pub station_name: Option<String>,
    pub line_name: Option<String>,
    pub operator_name: Option<String>,
    pub source_station_code: Option<String>,
    pub source_group_code: Option<String>,
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
    #[serde(default)]
    pub changed_fields: Vec<String>,
    pub before: Option<DatasetChangeVersionRefDto>,
    pub after: Option<DatasetChangeVersionRefDto>,
    pub station_name: Option<String>,
    pub line_name: Option<String>,
    pub operator_name: Option<String>,
    pub source_station_code: Option<String>,
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
    pub id: i64,
    pub snapshot_id: i64,
    pub source_version: Option<String>,
    pub station_uid: String,
    pub change_kind: DatasetChangeKindDto,
    pub station_name: Option<String>,
    pub line_name: Option<String>,
    pub operator_name: Option<String>,
    pub before_version_id: Option<i64>,
    pub after_version_id: Option<i64>,
    pub detail: DatasetChangeDetailDto,
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
    pub items: Vec<DatasetChangeEventDto>,
    pub limit: i64,
    pub snapshot_id: Option<i64>,
}
