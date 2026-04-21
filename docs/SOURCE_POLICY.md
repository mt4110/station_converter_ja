# SOURCE_POLICY

## Canonical source

v1 canonical source is MLIT / 国土数値情報 `N02`.
For the consumer-facing artifact license summary, see [DATA_LICENSE.md](./DATA_LICENSE.md).

Why:

- 全国一括
- 駅名 / 路線名 / 運営会社 / 駅コード / グループコードを持つ
- 2020年以降は CC BY 4.0
- 商用面で扱いやすい

## Optional source

`N05` is useful for historical open/close/rename tracking, but it has **non-commercial restrictions**.
Therefore:

- disabled by default
- treated as optional overlay
- never silently mixed into canonical export without policy review

## Geometry rule

- raw geometry stays as `geometry_geojson`
- `latitude` / `longitude` are derived representative points for search and APIs

## Station identity rule

Do not treat the upstream per-file code as your eternal primary key.  
Use repo-local `station_uid` and keep upstream codes as source attributes.
