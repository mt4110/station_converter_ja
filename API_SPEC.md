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

### GET `/v1/operators/{operator_name}/stations`

運営会社から駅一覧を返す。

## Notes

- `latitude` / `longitude` は代表点
- 原本 geometry は `geometry_geojson` に保持
- SQLite は read-only artifact 想定
