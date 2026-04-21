use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::schema::{
    ApiErrorCode, ApiErrorDetailDto, ApiErrorDetailPayloadDto, ApiErrorIssueDto,
    ApiErrorResponseDto, DatasetChangeDetailDto, DatasetChangeEventDto, DatasetChangeKindDto,
    DatasetChangeVersionRefDto, DatasetChangesResponseDto, DatasetSnapshotChangeCountsDto,
    DatasetSnapshotDto, DatasetSnapshotRefDto, DatasetSnapshotsResponseDto,
    DatasetStatusResponseDto, HealthResponseDto, LineCatalogItemDto, LineCatalogResponseDto,
    LineStationsResponseDto, NearbyStationsQueryDto, NearbyStationsResponseDto,
    OperatorStationsResponseDto, ReadinessDatasetDto, ReadinessResponseDto,
    StationSearchResponseDto, StationSummaryDto,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::health,
        crate::ready,
        crate::dataset_status,
        crate::dataset_snapshots,
        crate::dataset_changes,
        crate::search_stations,
        crate::nearby_stations,
        crate::line_catalog,
        crate::line_stations,
        crate::operator_stations
    ),
    components(schemas(
        ApiErrorCode,
        ApiErrorDetailDto,
        ApiErrorDetailPayloadDto,
        ApiErrorIssueDto,
        ApiErrorResponseDto,
        DatasetChangeDetailDto,
        DatasetChangeEventDto,
        DatasetChangeKindDto,
        DatasetChangeVersionRefDto,
        DatasetChangesResponseDto,
        DatasetSnapshotChangeCountsDto,
        DatasetSnapshotDto,
        DatasetSnapshotRefDto,
        DatasetSnapshotsResponseDto,
        DatasetStatusResponseDto,
        HealthResponseDto,
        LineCatalogItemDto,
        LineCatalogResponseDto,
        LineStationsResponseDto,
        NearbyStationsQueryDto,
        NearbyStationsResponseDto,
        OperatorStationsResponseDto,
        ReadinessDatasetDto,
        ReadinessResponseDto,
        StationSearchResponseDto,
        StationSummaryDto
    )),
    tags(
        (name = "station-api", description = "Public API backed by the latest available MLIT N02 snapshot.")
    )
)]
pub struct ApiDoc;

pub fn docs_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::from(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
}
