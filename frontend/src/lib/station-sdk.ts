const baseUrl = process.env.NEXT_PUBLIC_STATION_API_BASE_URL ?? "http://localhost:3212";

export async function searchStations(q: string, limit = 10) {
  const url = `${baseUrl}/v1/stations/search?q=${encodeURIComponent(q)}&limit=${limit}`;
  const response = await fetch(url);
  return response.json();
}

export async function searchNearbyStations(lat: number, lng: number, limit = 10) {
  const url = `${baseUrl}/v1/stations/nearby?lat=${lat}&lng=${lng}&limit=${limit}`;
  const response = await fetch(url);
  return response.json();
}
