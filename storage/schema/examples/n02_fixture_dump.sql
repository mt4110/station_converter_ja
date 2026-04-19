-- Lightweight fixture dump derived from testdata/n02.
-- This is a compact, human-readable sample for documentation.
-- It is intentionally small: 1 snapshot, 2 stations, 2 change events.

INSERT INTO source_snapshots (
  id,
  source_name,
  source_kind,
  source_version,
  source_url,
  source_sha256,
  downloaded_at
) VALUES (
  1,
  'ksj_n02_station',
  'geojson_zip_entry',
  'N02-24-fixture',
  'https://example.invalid/N02-24_GML.zip',
  '2ce1c63aeae3e6f2ae75e29f49737deee7d6a10a3dcd4c3de8464ef27b808520',
  '2026-01-01T00:00:00Z'
);

INSERT INTO station_identities (
  id,
  station_uid,
  canonical_name,
  created_at
) VALUES
  (
    1,
    'stn_n02_003568_e344d898ccc6422a',
    '中野',
    '2026-01-01T00:00:00Z'
  ),
  (
    2,
    'stn_n02_003700_49ed2e7fc9a4cd46',
    '新宿',
    '2026-01-01T00:00:00Z'
  );

INSERT INTO station_versions (
  id,
  station_uid,
  snapshot_id,
  source_station_code,
  source_group_code,
  station_name,
  line_name,
  operator_name,
  latitude,
  longitude,
  geometry_geojson,
  status,
  opened_on,
  closed_on,
  valid_from,
  valid_to,
  change_hash
) VALUES
  (
    1,
    'stn_n02_003568_e344d898ccc6422a',
    1,
    '003568',
    '003568',
    '中野',
    '中央線',
    '東日本旅客鉄道',
    35.7055,
    139.6655,
    '{"coordinates":[[139.665,35.705],[139.666,35.706]],"type":"LineString"}',
    'active',
    NULL,
    NULL,
    '2026-01-01T00:00:00Z',
    NULL,
    '4606df492972f4e36af89d7b430d9bfcd88206a1cf0551c170a70e15ad50f503'
  ),
  (
    2,
    'stn_n02_003700_49ed2e7fc9a4cd46',
    1,
    '003700',
    '003700',
    '新宿',
    '京王線',
    '京王電鉄',
    35.6910,
    139.7000,
    '{"coordinates":[[[139.699,35.69],[139.7,35.691]],[[139.7,35.691],[139.701,35.692]]],"type":"MultiLineString"}',
    'active',
    NULL,
    NULL,
    '2026-01-01T00:00:00Z',
    NULL,
    'a35a16fb996ce96e5c7cf88cb218b1ddf1829c318e3343d220fb7df8cf14f78e'
  );

INSERT INTO station_change_events (
  id,
  snapshot_id,
  station_uid,
  change_kind,
  before_version_id,
  after_version_id,
  detail_json,
  created_at
) VALUES
  (
    1,
    1,
    'stn_n02_003568_e344d898ccc6422a',
    'created',
    NULL,
    1,
    '{"line_name":"中央線","operator_name":"東日本旅客鉄道","source_group_code":"003568","source_station_code":"003568","station_name":"中野"}',
    '2026-01-01T00:00:00Z'
  ),
  (
    2,
    1,
    'stn_n02_003700_49ed2e7fc9a4cd46',
    'created',
    NULL,
    2,
    '{"line_name":"京王線","operator_name":"京王電鉄","source_group_code":"003700","source_station_code":"003700","station_name":"新宿"}',
    '2026-01-01T00:00:00Z'
  );
