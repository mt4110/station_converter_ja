# DATABASE

## What lands in the database

この repo の write path は、N02 の年次スナップショットを取り込み、
次の 4 テーブルと 1 view に正規化します。

- `source_snapshots`
  - どの upstream ZIP を取り込んだか
- `station_identities`
  - repo 内での永続 station id
- `station_versions`
  - snapshot ごとの immutable version row
- `station_change_events`
  - `created / updated / removed` の差分イベント
- `stations_latest`
  - 現在有効な station row だけを見る view

関係は次のとおりです。

```text
source_snapshots (1) ----< station_versions >---- (1) station_identities
        |
        +----< station_change_events

stations_latest = SELECT * FROM station_versions WHERE valid_to IS NULL
```

## DDL

DB ごとの初期 DDL は migration にあります。

- PostgreSQL: [`storage/migrations/postgres/0001_init.sql`](../storage/migrations/postgres/0001_init.sql)
- MySQL: [`storage/migrations/mysql/0001_init.sql`](../storage/migrations/mysql/0001_init.sql)
- SQLite: [`storage/migrations/sqlite/0001_init.sql`](../storage/migrations/sqlite/0001_init.sql)

## Table Guide

### `source_snapshots`

取り込んだ upstream archive の出自を持ちます。

- `source_name`
- `source_kind`
- `source_version`
- `source_url`
- `source_sha256`
- `downloaded_at`

### `station_identities`

駅そのものの永続 id を持ちます。

- `station_uid`
- `canonical_name`
- `created_at`

### `station_versions`

1 snapshot における駅の状態本体です。

- `station_uid`
- `snapshot_id`
- `source_station_code`
- `source_group_code`
- `station_name`
- `line_name`
- `operator_name`
- `latitude`
- `longitude`
- `geometry_geojson`
- `status`
- `opened_on`
- `closed_on`
- `valid_from`
- `valid_to`
- `change_hash`

`latitude` / `longitude` は検索向け代表点で、元の線形 geometry は `geometry_geojson` に残します。

### `station_change_events`

snapshot ごとの差分イベントを持ちます。

- `snapshot_id`
- `station_uid`
- `change_kind`
- `before_version_id`
- `after_version_id`
- `detail_json`
- `created_at`

## Lightweight Sample Dump

最小 fixture から切り出した軽い SQL dump を同梱しています。

- [`storage/schema/examples/n02_fixture_dump.sql`](../storage/schema/examples/n02_fixture_dump.sql)

これは 1 snapshot / 2 station の小さい例です。

- `中野`
- `新宿`

## Example Queries

### latest station rows

```sql
SELECT
  station_uid,
  station_name,
  line_name,
  operator_name,
  latitude,
  longitude,
  status
FROM stations_latest
ORDER BY operator_name, line_name, station_name
LIMIT 20;
```

### snapshot history

```sql
SELECT
  id,
  source_name,
  source_version,
  source_url,
  source_sha256,
  downloaded_at
FROM source_snapshots
ORDER BY id DESC;
```

### recent change events

```sql
SELECT
  sce.id,
  sce.change_kind,
  sv.station_name,
  sv.line_name,
  sv.operator_name,
  sce.created_at
FROM station_change_events AS sce
LEFT JOIN station_versions AS sv
  ON sv.id = sce.after_version_id
ORDER BY sce.id DESC
LIMIT 20;
```

### current rows for one station name

```sql
SELECT
  station_uid,
  station_name,
  line_name,
  operator_name,
  latitude,
  longitude
FROM stations_latest
WHERE station_name = '新宿'
ORDER BY operator_name, line_name;
```

## Notes

- `stations_latest` は convenience view です。差分追跡の source of truth は `station_versions` です。
- SQLite artifact は read-only 配布物です。primary write DB には PostgreSQL / MySQL を使います。
- 代表点と raw geometry を分けている理由は [`storage/schema/README.md`](../storage/schema/README.md) にあります。
