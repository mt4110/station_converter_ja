import { NextRequest, NextResponse } from "next/server";

const ADDRESS_SEARCH_URL = "https://msearch.gsi.go.jp/address-search/AddressSearch";
const MAX_QUERY_LENGTH = 120;

type AddressCandidate = {
  title: string;
  latitude: number;
  longitude: number;
};

type GsiFeature = {
  geometry?: {
    coordinates?: unknown;
  };
  properties?: {
    title?: unknown;
  };
};

function normalizedLimit(rawLimit: string | null) {
  const parsed = Number(rawLimit ?? "5");

  if (!Number.isFinite(parsed)) {
    return 5;
  }

  return Math.min(8, Math.max(1, Math.trunc(parsed)));
}

function normalizeQuery(rawQuery: string) {
  return rawQuery.normalize("NFKC").replace(/[ \t\r\n　]+/g, "");
}

function extractMunicipalityQuery(query: string) {
  const prefectureMatch = query.match(/^(東京都|北海道|(?:京都|大阪)府|.+?県)/);

  if (!prefectureMatch) {
    return null;
  }

  const prefecture = prefectureMatch[0];
  const rest = query.slice(prefecture.length);
  const municipalityMatch = rest.match(/^(.+?(?:市.+?区|郡.+?(?:町|村)|(?:市|区|町|村)))/);

  if (!municipalityMatch) {
    return null;
  }

  return `${prefecture}${municipalityMatch[0]}`;
}

function toAddressCandidates(payload: unknown, limit: number) {
  if (!Array.isArray(payload)) {
    return [];
  }

  const items: AddressCandidate[] = [];
  const seen = new Set<string>();

  for (const feature of payload as GsiFeature[]) {
    const coordinates = feature.geometry?.coordinates;
    const title = feature.properties?.title;

    if (!Array.isArray(coordinates) || coordinates.length < 2 || typeof title !== "string" || title.length === 0) {
      continue;
    }

    const [longitude, latitude] = coordinates;

    if (typeof latitude !== "number" || typeof longitude !== "number") {
      continue;
    }

    const key = `${title}:${latitude}:${longitude}`;

    if (seen.has(key)) {
      continue;
    }

    seen.add(key);
    items.push({ title, latitude, longitude });

    if (items.length >= limit) {
      break;
    }
  }

  return items;
}

async function fetchCandidates(query: string, limit: number) {
  const response = await fetch(`${ADDRESS_SEARCH_URL}?q=${encodeURIComponent(query)}`, {
    cache: "no-store",
    signal: AbortSignal.timeout(3000)
  });

  if (!response.ok) {
    throw new Error(`address_search_failed:${response.status}`);
  }

  return toAddressCandidates(await response.json(), limit);
}

export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const limit = normalizedLimit(searchParams.get("limit"));
  const query = normalizeQuery(searchParams.get("q") ?? "");

  if (query.length === 0) {
    return NextResponse.json({
      items: [],
      limit,
      query,
      resolved_query: query,
      fallback_used: false
    });
  }

  if (query.length > MAX_QUERY_LENGTH) {
    return NextResponse.json(
      {
        error: "query_too_long",
        max_query_length: MAX_QUERY_LENGTH
      },
      {
        status: 400
      }
    );
  }

  const municipalityQuery = extractMunicipalityQuery(query);
  const attempts = municipalityQuery && municipalityQuery !== query ? [query, municipalityQuery] : [query];

  try {
    for (let index = 0; index < attempts.length; index += 1) {
      const attempt = attempts[index];
      const items = await fetchCandidates(attempt, limit);

      if (items.length > 0) {
        return NextResponse.json({
          items,
          limit,
          query,
          resolved_query: attempt,
          fallback_used: attempt !== query && index > 0
        });
      }
    }

    return NextResponse.json({
      items: [],
      limit,
      query,
      resolved_query: query,
      fallback_used: false
    });
  } catch {
    return NextResponse.json(
      {
        error: "address_search_unavailable"
      },
      {
        status: 502
      }
    );
  }
}
