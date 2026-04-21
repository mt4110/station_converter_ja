# API_SPEC

## Base URL

- local API: `http://localhost:3212`
- OpenAPI JSON: `http://localhost:3212/openapi.json`
- docs UI: `http://localhost:3212/docs` (`/docs/` へ redirect)

## Contract Source

- machine-readable な canonical contract は `worker/api` から生成される `/openapi.json`
- この doc は同じ contract を読むための human-readable companion
- coverage 対象は `station-api` の public endpoint のみ
- frontend の `/api/address-search` は hand-written な Next.js helper route で、国土地理院 Address Search を使う example frontend 専用導線
- そのため `/api/address-search` は OpenAPI / generated station SDK の対象外

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

DB に接続できない場合は `503` で同 shape を返します。

### GET `/v1/dataset/status`

現在アクティブな駅データ件数と、sample web を出してよい状態かどうかを返す。
`active_snapshot` は N02 について直近で取り込まれた snapshot metadata です。
この endpoint は最新の public claim を確認する入口ですが、real-time railway data ではなく
latest ingested MLIT N02 snapshot metadata を含む状態を返します。

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

### GET `/v1/dataset/snapshots?limit=20`

canonical N02 dataset の snapshot history を返す。
新しい snapshot から順に返り、`limit` は `1..=200` に正規化されます。
`id` は `/v1/dataset/changes?snapshot_id=...` に渡せる source snapshot id です。
`station_version_count` と `change_counts` は N02 station identity に scope されます。
`source_version` は source package から導出できない場合 `null` になり得ます。
`downloaded_at` は active database dialect が返す timestamp string です。
v1 では PostgreSQL / MySQL / SQLite 間で RFC3339 へ正規化していません。

```json
{
  "items": [
    {
      "id": 25,
      "source_name": "ksj_n02_station",
      "source_kind": "geojson_zip_entry",
      "source_version": "N02-25",
      "source_url": "https://example.com/N02-25_GML.zip",
      "source_sha256": "84d675d10bfe01b7fdcbe97cf9221c0b5054d5833cf9a339b37e8b82ac3bd5aa",
      "downloaded_at": "2026-04-20 12:34:56",
      "station_version_count": 10155,
      "change_counts": {
        "created": 12,
        "updated": 4,
        "removed": 1,
        "total": 17
      }
    }
  ],
  "limit": 20
}
```

### GET `/v1/dataset/changes?limit=20&snapshot_id=25`

canonical N02 dataset の recent change events を返す。
`snapshot_id` を付けると、その snapshot に絞り込める。
新しい change event から順に返り、`limit` は `1..=200` に正規化されます。
存在しない `snapshot_id` は `404 not_found` です。

`change_kind` の読み方:

- `created`: その snapshot で station identity が初めて出現した
- `updated`: 既存 station identity の tracked field が変わった
- `removed`: 前 snapshot まで存在した station identity が、その snapshot では見つからなかった

`station_name` / `line_name` / `operator_name` は一覧表示しやすい best-effort context です。
field-level diff が必要な consumer は `detail.before` / `detail.after` / `detail.changed_fields` を使います。
現在の detail payload では、`updated` は `before` / `after` と `changed_fields` を持ちます。
`created` / `removed` は flat context fields を持つため、consumer は top-level context fields も fallback として扱ってください。
`created_at` も active database dialect が返す timestamp string で、v1 では RFC3339 正規化していません。

```json
{
  "items": [
    {
      "id": 42,
      "snapshot_id": 25,
      "source_version": "N02-25",
      "station_uid": "stn_n02_003700_49ed2e7fc9a4cd46",
      "change_kind": "updated",
      "station_name": "新宿",
      "line_name": "京王新線",
      "operator_name": "京王電鉄",
      "before_version_id": 10,
      "after_version_id": 11,
      "detail": {
        "changed_fields": ["line_name"],
        "before": {
          "station_name": "新宿",
          "line_name": "京王線",
          "operator_name": "京王電鉄",
          "source_station_code": "003700",
          "source_group_code": "003700",
          "status": "active"
        },
        "after": {
          "station_name": "新宿",
          "line_name": "京王新線",
          "operator_name": "京王電鉄",
          "source_station_code": "003700",
          "source_group_code": "003700",
          "status": "active"
        }
      },
      "created_at": "2026-04-20 12:34:56"
    }
  ],
  "limit": 20,
  "snapshot_id": 25
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
  ],
  "limit": 10,
  "query": "新宿"
}
```

### GET `/v1/stations/nearby?lat=35.6895&lng=139.6917&limit=10`

代表点ベースの近傍検索。  
v1 は `latitude` / `longitude` による検索。将来は geometry と bbox を併用。

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
  ],
  "limit": 10,
  "query": {
    "lat": 35.6895,
    "lng": 139.6917
  }
}
```

### GET `/v1/lines/{line_name}/stations`

路線名から駅一覧を返す。

`operator_name` を付けると、同名路線を事業者で絞り込める。

```json
{
  "line_name": "中央線",
  "operator_name": "東日本旅客鉄道",
  "items": [
    {
      "station_uid": "stn_jp_example_shinjuku",
      "station_name": "新宿",
      "line_name": "中央線",
      "operator_name": "東日本旅客鉄道",
      "latitude": 35.6909,
      "longitude": 139.7003,
      "status": "active"
    }
  ]
}
```

### GET `/v1/operators/{operator_name}/stations`

運営会社から駅一覧を返す。

```json
{
  "operator_name": "東日本旅客鉄道",
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

## Error response

query parameter validation failure は `400`、unknown `snapshot_id` は `404`、unexpected failure は `500` を返します。
現行の共通 error envelope は `error.code` / `error.message` のみです。
field-level validation detail が必要になった場合は、将来 optional `error.detail` として追加します。
その場合も既存 consumer は `code` / `message` を読み続けられる形にします。

```json
{
  "error": {
    "code": "invalid_request",
    "message": "Failed to deserialize query string"
  }
}
```

## Notes

- `latitude` / `longitude` は代表点
- 原本 geometry は `geometry_geojson` に保持
- SQLite は read-only artifact 想定
- public freshness claim は latest available MLIT N02 snapshot
- `/api/address-search` は example frontend 用の補助 endpoint であり、public `station-api` contract には含めない
- machine-readable な canonical contract は `/openapi.json`
- human-readable な補助説明はこの `API_SPEC.md` と `docs/OPENAPI.md`
