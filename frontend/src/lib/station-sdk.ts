const baseUrl = process.env.NEXT_PUBLIC_STATION_API_BASE_URL ?? "http://localhost:3212";

class ApiError extends Error {
  status: number;

  constructor(status: number) {
    super(`API request failed: ${status}`);
    this.name = "ApiError";
    this.status = status;
  }
}

export type StationSummary = {
  station_uid: string;
  station_name: string;
  line_name: string;
  operator_name: string;
  latitude: number;
  longitude: number;
  status: string;
};

export type StationSearchResponse = {
  items: StationSummary[];
  limit: number;
  query: string;
};

export type NearbyStationsResponse = {
  items: StationSummary[];
  limit: number;
  query: {
    lat: number;
    lng: number;
  };
};

export type LineStationsResponse = {
  items: StationSummary[];
  line_name: string;
};

export type LineCatalogEntry = {
  line_name: string;
  operator_name: string;
  station_count: number;
};

export type LineCatalogResponse = {
  items: LineCatalogEntry[];
  limit: number;
  query: string;
};

export type OperatorStationsResponse = {
  items: StationSummary[];
  operator_name: string;
};

export type AddressCandidate = {
  title: string;
  latitude: number;
  longitude: number;
};

export type AddressSearchResponse = {
  items: AddressCandidate[];
  limit: number;
  query: string;
  resolved_query: string;
  fallback_used: boolean;
};

export type DatasetStatus = {
  status: "ready" | "needs_ingest";
  looks_like_full_dataset: boolean;
  can_query_stations: boolean;
  status_source: "dataset_status" | "legacy_search_probe";
  source_is_local: boolean;
  active_station_count: number | null;
  distinct_station_name_count: number | null;
  distinct_line_count: number | null;
  active_snapshot_count: number | null;
  active_snapshot: {
    id?: number | null;
    source_version?: string | null;
    source_url?: string | null;
  } | null;
};

type DatasetStatusPayload = Omit<DatasetStatus, "can_query_stations" | "status_source">;

type LegacyStationSearchProbe = {
  items?: unknown;
};

async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url);

  if (!response.ok) {
    throw new ApiError(response.status);
  }

  return response.json() as Promise<T>;
}

async function probeLegacyDatasetStatus() {
  const url = `${baseUrl}/v1/stations/search?q=${encodeURIComponent("新宿")}&limit=1`;
  const payload = await fetchJson<LegacyStationSearchProbe>(url);

  if (!Array.isArray(payload.items)) {
    throw new Error("Legacy station API probe returned an invalid response.");
  }

  return {
    status: "ready",
    looks_like_full_dataset: false,
    can_query_stations: true,
    status_source: "legacy_search_probe",
    source_is_local: false,
    active_station_count: null,
    distinct_station_name_count: null,
    distinct_line_count: null,
    active_snapshot_count: null,
    active_snapshot: null
  } satisfies DatasetStatus;
}

export async function searchStations(q: string, limit = 10) {
  const url = `${baseUrl}/v1/stations/search?q=${encodeURIComponent(q)}&limit=${limit}`;
  return fetchJson<StationSearchResponse>(url);
}

export async function searchNearbyStations(lat: number, lng: number, limit = 10) {
  const url = `${baseUrl}/v1/stations/nearby?lat=${lat}&lng=${lng}&limit=${limit}`;
  return fetchJson<NearbyStationsResponse>(url);
}

export async function listLineStations(lineName: string) {
  const url = `${baseUrl}/v1/lines/${encodeURIComponent(lineName)}/stations`;
  return fetchJson<LineStationsResponse>(url);
}

export async function listLineCatalog(q = "", limit = 60) {
  const url = `${baseUrl}/v1/lines/catalog?q=${encodeURIComponent(q)}&limit=${limit}`;
  return fetchJson<LineCatalogResponse>(url);
}

export async function listOperatorStations(operatorName: string) {
  const url = `${baseUrl}/v1/operators/${encodeURIComponent(operatorName)}/stations`;
  return fetchJson<OperatorStationsResponse>(url);
}

export async function searchAddressCandidates(q: string, limit = 5) {
  const url = `/api/address-search?q=${encodeURIComponent(q)}&limit=${limit}`;
  return fetchJson<AddressSearchResponse>(url);
}

export async function getDatasetStatus() {
  const url = `${baseUrl}/v1/dataset/status`;

  try {
    const payload = await fetchJson<DatasetStatusPayload>(url);
    return {
      ...payload,
      can_query_stations: payload.looks_like_full_dataset,
      status_source: "dataset_status"
    } satisfies DatasetStatus;
  } catch (error) {
    if (error instanceof ApiError && error.status === 404) {
      return probeLegacyDatasetStatus();
    }

    throw error;
  }
}
