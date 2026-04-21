import {
  ApiError,
  getDatasetStatus as getDatasetStatusFromApi,
  searchStations,
  type DatasetStatusResponse
} from "./station-sdk";

export type DatasetStatus = Omit<
  DatasetStatusResponse,
  | "active_station_count"
  | "distinct_station_name_count"
  | "distinct_line_count"
  | "active_version_snapshot_count"
> & {
  active_station_count: number | null;
  distinct_station_name_count: number | null;
  distinct_line_count: number | null;
  active_version_snapshot_count: number | null;
  can_query_stations: boolean;
  status_source: "dataset_status" | "legacy_search_probe";
};

export function toDatasetStatus(payload: DatasetStatusResponse): DatasetStatus {
  return {
    ...payload,
    can_query_stations: payload.looks_like_full_dataset,
    status_source: "dataset_status"
  };
}

export async function loadDatasetStatus() {
  try {
    return toDatasetStatus(await getDatasetStatusFromApi());
  } catch (error) {
    if (!(error instanceof ApiError) || error.status !== 404) {
      throw error;
    }

    const probe = await searchStations("新宿", 1);
    const canQueryStations = probe.items.length > 0;

    return {
      status: canQueryStations ? "ready" : "needs_ingest",
      looks_like_full_dataset: false,
      can_query_stations: canQueryStations,
      status_source: "legacy_search_probe",
      source_is_local: false,
      active_station_count: null,
      distinct_station_name_count: null,
      distinct_line_count: null,
      active_version_snapshot_count: null,
      active_snapshot: null
    } satisfies DatasetStatus;
  }
}
