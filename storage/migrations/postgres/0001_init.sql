
CREATE TABLE IF NOT EXISTS source_snapshots (
  id BIGSERIAL PRIMARY KEY,
  source_name TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_version TEXT,
  source_url TEXT NOT NULL,
  source_sha256 TEXT NOT NULL,
  downloaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (source_name, source_sha256)
);

CREATE TABLE IF NOT EXISTS station_identities (
  id BIGSERIAL PRIMARY KEY,
  station_uid TEXT NOT NULL UNIQUE,
  canonical_name TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS station_versions (
  id BIGSERIAL PRIMARY KEY,
  station_uid TEXT NOT NULL REFERENCES station_identities (station_uid),
  snapshot_id BIGINT NOT NULL REFERENCES source_snapshots (id),
  source_station_code TEXT,
  source_group_code TEXT,
  station_name TEXT NOT NULL,
  line_name TEXT NOT NULL,
  operator_name TEXT NOT NULL,
  latitude DOUBLE PRECISION NOT NULL,
  longitude DOUBLE PRECISION NOT NULL,
  geometry_geojson TEXT,
  status TEXT NOT NULL DEFAULT 'active',
  opened_on DATE,
  closed_on DATE,
  valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  valid_to TIMESTAMPTZ,
  change_hash TEXT NOT NULL,
  UNIQUE (station_uid, snapshot_id)
);

CREATE INDEX IF NOT EXISTS idx_station_versions_station_uid ON station_versions (station_uid);
CREATE INDEX IF NOT EXISTS idx_station_versions_station_name ON station_versions (station_name);
CREATE INDEX IF NOT EXISTS idx_station_versions_line_name ON station_versions (line_name);
CREATE INDEX IF NOT EXISTS idx_station_versions_operator_name ON station_versions (operator_name);
CREATE INDEX IF NOT EXISTS idx_station_versions_source_station_code ON station_versions (source_station_code);
CREATE INDEX IF NOT EXISTS idx_station_versions_lat_lng ON station_versions (latitude, longitude);

CREATE TABLE IF NOT EXISTS station_change_events (
  id BIGSERIAL PRIMARY KEY,
  snapshot_id BIGINT NOT NULL REFERENCES source_snapshots (id),
  station_uid TEXT NOT NULL REFERENCES station_identities (station_uid),
  change_kind TEXT NOT NULL,
  before_version_id BIGINT,
  after_version_id BIGINT,
  detail_json TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE OR REPLACE VIEW stations_latest AS
SELECT *
FROM station_versions
WHERE valid_to IS NULL;
