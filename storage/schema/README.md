# storage/schema

## Design

- `source_snapshots`
  - upstream archive / checksum / version metadata
- `station_identities`
  - repo 内での永続 ID (`station_uid`)
- `station_versions`
  - immutable version rows
- `station_change_events`
  - add / rename / relocate / regroup / close などの差分イベント

テーブル構造の見取り図、軽い sample dump、example query は
[`docs/DATABASE.md`](../../docs/DATABASE.md) を参照。

## Latitude / Longitude

国土数値情報 N02 は駅 geometry を線として保持している年次があるため、  
検索用の `latitude` / `longitude` は **代表点** として別列で持つ。  
raw geometry は `geometry_geojson` に残す。

## Why not PostGIS first?

- PostgreSQL / MySQL / SQLite を並立させたい
- local 開発の再現性を保ちたい
- SQLite artifact を素直に配りたい

なので v1 は **DB-agnostic に寄せる**。

## SQLite artifact export

`station-ops export-sqlite` は primary write DB から SQLite に全テーブルを複製し、
read-only artifact として `storage/sqlite/stations.sqlite3` を生成する。

- `source_snapshots`
- `station_identities`
- `station_versions`
- `station_change_events`

配布用のコピーと checksum / manifest は `scripts/package_sqlite_release.sh` で作る。
配布まで含めた公式導線は `scripts/release_sqlite_artifact.sh`。
