# CONTAINER_IMAGES

## Images

GHCR には次の 2 つの image を publish します。

- `ghcr.io/mt4110/station_converter_ja/api`
  - resident API service
  - default command: `station-api`
- `ghcr.io/mt4110/station_converter_ja/ops`
  - operational one-shot workflow
  - entrypoint: `station-ops`

本番 ingest の入口は `station-ops job ingest-n02` です。
`station-crawler` は dev helper であり、本番常駐 worker として扱いません。

## Publish policy

`.github/workflows/container-images.yml` が image build / publish を担当します。

- pull request: build only
- push to `main`: publish `sha-<short-sha>` tag
- push to `v*` tag: publish `sha-<short-sha>` tag and release tag

この workflow は GHCR への image publish だけを行います。
cloud production resource は作成しません。

## Local image build

publish 前に手元で build だけ確認する場合:

```bash
docker build -f worker/api/Dockerfile -t station-converter-ja-api:local .
docker build -f worker/ops/Dockerfile -t station-converter-ja-ops:local .
```

## Minimal run with Docker Compose DB

PostgreSQL を primary write DB として起動します。
SQLite は read-only artifact の出力先であり、primary write DB として使いません。

```bash
docker compose --project-name station-converter-ja up -d postgres
```

published image を使う場合は tag を指定します。
`main` 由来なら `sha-<short-sha>`、release 由来なら `v0.1.5` のような tag です。

```bash
IMAGE_TAG=sha-<short-sha>
```

migrate:

```bash
docker run --rm \
  --network station-converter-ja_default \
  -e DATABASE_TYPE=postgres \
  -e POSTGRES_DATABASE_URL=postgres://postgres:postgres_password@postgres:5432/station_db \
  ghcr.io/mt4110/station_converter_ja/ops:${IMAGE_TAG} \
  migrate
```

N02 ingest:

```bash
docker run --rm \
  --network station-converter-ja_default \
  -e DATABASE_TYPE=postgres \
  -e POSTGRES_DATABASE_URL=postgres://postgres:postgres_password@postgres:5432/station_db \
  ghcr.io/mt4110/station_converter_ja/ops:${IMAGE_TAG} \
  job ingest-n02
```

API:

```bash
docker run --rm \
  --network station-converter-ja_default \
  -p 3212:3212 \
  -e DATABASE_TYPE=postgres \
  -e POSTGRES_DATABASE_URL=postgres://postgres:postgres_password@postgres:5432/station_db \
  -e BIND_ADDR=0.0.0.0:3212 \
  ghcr.io/mt4110/station_converter_ja/api:${IMAGE_TAG}
```

Readiness:

```bash
curl http://127.0.0.1:3212/ready
```

Redis は optional cache only です。将来的に cache-backed path を使う場合は
`REDIS_URL` を設定できますが、Redis を source of truth として扱ってはいけません。

## SQLite artifact export

SQLite export は primary DB からつなぐ artifact flow のままにします。

```bash
docker run --rm \
  --user "$(id -u):$(id -g)" \
  --network station-converter-ja_default \
  -v "$PWD/storage/sqlite:/app/storage/sqlite" \
  -e DATABASE_TYPE=postgres \
  -e POSTGRES_DATABASE_URL=postgres://postgres:postgres_password@postgres:5432/station_db \
  -e SQLITE_DATABASE_URL=sqlite:///app/storage/sqlite/stations.sqlite3 \
  -e JOB_LOCK_DIR=/tmp/station-locks \
  -e TEMP_ASSET_DIR=/tmp/station-temp-assets \
  ghcr.io/mt4110/station_converter_ja/ops:${IMAGE_TAG} \
  job ingest-n02 --export-sqlite
```

この command でも canonical source は MLIT `N02`、primary write DB は PostgreSQL です。
生成された SQLite は配布用の read-only artifact として扱います。
