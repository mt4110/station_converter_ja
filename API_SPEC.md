# API_SPEC

## Base URL

- local API: `http://localhost:3212`

## Endpoints

### GET `/health`

Liveness check.

```json
{ "status": "ok", "service": "station-api" }
```

### GET `/ready`

Readiness check.

```json
{
  "status": "ready",
  "database_type": "postgres",
  "cache": "disabled"
}
```

### GET `/v1/dataset/status`

現在アクティブな駅データ件数と、sample web を出してよい状態かどうかを返す。

```json
{
  "status": "ready",
  "looks_like_full_dataset": true,
  "source_is_local": false,
  "active_station_count": 10155,
  "distinct_station_name_count": 9102,
  "distinct_line_count": 623,
  "active_version_snapshot_count": 2,
  "active_snapshot": {
    "id": 25,
    "source_version": "N02-25",
    "source_url": "https://example.com/N02-25_GML.zip"
  }
}
```

### GET `/v1/lines/catalog?q=中央&limit=20`

路線名の候補一覧を返す。`q` は部分一致、`limit` は最大件数。

```json
{
  "items": [
    {
      "line_name": "中央線",
      "operator_name": "東日本旅客鉄道",
      "station_count": 24
    },
    {
      "line_name": "中央線",
      "operator_name": "東京地下鉄",
      "station_count": 6
    }
  ],
  "limit": 20,
  "query": "中央"
}
```

### GET `/v1/stations/search?q=新宿&limit=10`

駅名の前方一致 / 部分一致検索。

```json
{
  "items": [
    {
      "station_uid": "stn_jp_example_shinjuku",
      "station_name": "新宿",
      "line_name": "山手線",
      "operator_name": "東日本旅客鉄道",
      "latitude": 35.6909,
      "longitude": 139.7003,
      "status": "active"
    }
  ]
}
```

### GET `/v1/stations/nearby?lat=35.6895&lng=139.6917&limit=10`

代表点ベースの近傍検索。  
v1 は `latitude` / `longitude` による検索。将来は geometry と bbox を併用。

### GET `/v1/lines/{line_name}/stations`

路線名から駅一覧を返す。

`operator_name` を付けると、同名路線を事業者で絞り込める。

### GET `/v1/operators/{operator_name}/stations`

運営会社から駅一覧を返す。

## Notes

- `latitude` / `longitude` は代表点
- 原本 geometry は `geometry_geojson` に保持
- SQLite は read-only artifact 想定
