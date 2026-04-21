import { ApiError } from "./station-sdk";

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

async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url);

  if (!response.ok) {
    throw new ApiError(response.status);
  }

  return response.json() as Promise<T>;
}

export async function searchAddressCandidates(q: string, limit = 5) {
  const url = `/api/address-search?q=${encodeURIComponent(q)}&limit=${limit}`;
  return fetchJson<AddressSearchResponse>(url);
}
