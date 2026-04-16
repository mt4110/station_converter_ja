
CREATE TABLE IF NOT EXISTS source_snapshots (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  source_name TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_version TEXT NULL,
  source_url TEXT NOT NULL,
  source_sha256 VARCHAR(128) NOT NULL,
  downloaded_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uq_source_snapshots_name_sha (source_name(128), source_sha256)
);

CREATE TABLE IF NOT EXISTS station_identities (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  station_uid VARCHAR(255) NOT NULL,
  canonical_name TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uq_station_uid (station_uid)
);

CREATE TABLE IF NOT EXISTS station_versions (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  station_uid VARCHAR(255) NOT NULL,
  snapshot_id BIGINT UNSIGNED NOT NULL,
  source_station_code VARCHAR(255) NULL,
  source_group_code VARCHAR(255) NULL,
  station_name TEXT NOT NULL,
  line_name TEXT NOT NULL,
  operator_name TEXT NOT NULL,
  latitude DOUBLE NOT NULL,
  longitude DOUBLE NOT NULL,
  geometry_geojson LONGTEXT NULL,
  status VARCHAR(32) NOT NULL DEFAULT 'active',
  opened_on DATE NULL,
  closed_on DATE NULL,
  valid_from TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  valid_to TIMESTAMP NULL,
  change_hash VARCHAR(128) NOT NULL,
  UNIQUE KEY uq_station_versions_station_snapshot (station_uid, snapshot_id),
  KEY idx_station_versions_station_uid (station_uid),
  KEY idx_station_versions_source_station_code (source_station_code),
  KEY idx_station_versions_lat_lng (latitude, longitude),
  CONSTRAINT fk_station_versions_station_uid FOREIGN KEY (station_uid) REFERENCES station_identities (station_uid),
  CONSTRAINT fk_station_versions_snapshot_id FOREIGN KEY (snapshot_id) REFERENCES source_snapshots (id)
);

CREATE TABLE IF NOT EXISTS station_change_events (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  snapshot_id BIGINT UNSIGNED NOT NULL,
  station_uid VARCHAR(255) NOT NULL,
  change_kind VARCHAR(64) NOT NULL,
  before_version_id BIGINT UNSIGNED NULL,
  after_version_id BIGINT UNSIGNED NULL,
  detail_json LONGTEXT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY idx_station_change_events_station_uid (station_uid),
  CONSTRAINT fk_station_change_events_snapshot_id FOREIGN KEY (snapshot_id) REFERENCES source_snapshots (id),
  CONSTRAINT fk_station_change_events_station_uid FOREIGN KEY (station_uid) REFERENCES station_identities (station_uid)
);

CREATE OR REPLACE VIEW stations_latest AS
SELECT *
FROM station_versions
WHERE valid_to IS NULL;
