
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS source_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_name TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_version TEXT,
  source_url TEXT NOT NULL,
  source_sha256 TEXT NOT NULL,
  downloaded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (source_name, source_sha256)
);

CREATE TABLE IF NOT EXISTS station_identities (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  station_uid TEXT NOT NULL UNIQUE,
  canonical_name TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS station_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  station_uid TEXT NOT NULL,
  snapshot_id INTEGER NOT NULL,
  source_station_code TEXT,
  source_group_code TEXT,
  station_name TEXT NOT NULL,
  line_name TEXT NOT NULL,
  operator_name TEXT NOT NULL,
  latitude REAL NOT NULL,
  longitude REAL NOT NULL,
  geometry_geojson TEXT,
  status TEXT NOT NULL DEFAULT 'active',
  opened_on TEXT,
  closed_on TEXT,
  valid_from TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  valid_to TEXT,
  change_hash TEXT NOT NULL,
  UNIQUE (station_uid, snapshot_id),
  FOREIGN KEY (station_uid) REFERENCES station_identities (station_uid),
  FOREIGN KEY (snapshot_id) REFERENCES source_snapshots (id)
);

CREATE INDEX IF NOT EXISTS idx_station_versions_station_uid ON station_versions (station_uid);
CREATE INDEX IF NOT EXISTS idx_station_versions_station_name ON station_versions (station_name);
CREATE INDEX IF NOT EXISTS idx_station_versions_line_name ON station_versions (line_name);
CREATE INDEX IF NOT EXISTS idx_station_versions_operator_name ON station_versions (operator_name);
CREATE INDEX IF NOT EXISTS idx_station_versions_source_station_code ON station_versions (source_station_code);
CREATE INDEX IF NOT EXISTS idx_station_versions_lat_lng ON station_versions (latitude, longitude);

CREATE TABLE IF NOT EXISTS station_change_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  snapshot_id INTEGER NOT NULL,
  station_uid TEXT NOT NULL,
  change_kind TEXT NOT NULL,
  before_version_id INTEGER,
  after_version_id INTEGER,
  detail_json TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (snapshot_id) REFERENCES source_snapshots (id),
  FOREIGN KEY (station_uid) REFERENCES station_identities (station_uid)
);

CREATE VIEW IF NOT EXISTS stations_latest AS
SELECT *
FROM station_versions
WHERE valid_to IS NULL;
