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
API・ops・crawler の実行イメージは、ダイジェストで固定した Chainguard static を使用します。
コンテナ内の操作には各バイナリを直接指定します。

## Publish policy

`.github/workflows/container-images.yml` が image build / publish を担当します。

- pull request: ビルドと OSV 検査
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

Compose の PostgreSQL、Redis、Adminer は `deploy/docker/` の Dockerfile からビルドします。
ベースイメージを取得し直してパッケージ更新を適用するときは、対象サービスを再ビルドします。

```bash
docker compose --profile cache --profile dbweb build --pull redis adminer
```

CI と SQLite リリース用の一時 PostgreSQL は、更新済みイメージの OSV 検査が成功してから起動します。
CI 用の一時 MySQL も更新済みイメージを使い、Oracle Linux の RPM を Trivy で検査してから起動します。
配布元イメージも依存関係の必須チェックで検査します。

## Minimal run with Docker Compose DB

PostgreSQL を primary write DB として起動します。
SQLite は read-only artifact の出力先であり、primary write DB として使いません。

```bash
docker compose --project-name station-converter-ja up -d postgres
```

### 既存の Debian 版 PostgreSQL 16 から Alpine 版へ移行

Alpine 版は新しい `postgres_alpine_data` ボリュームを使います。
`docker-compose.local.yml` を併用する場合は `.docker-data/postgres-alpine` が新しい保存先です。
既存の Debian データを移行する場合は、アプリと取り込みジョブからの書き込みを止め、
同じ Compose プロジェクトと構成ファイルを指定して以下を実行します。

```bash
umask 077
backup="$(mktemp "${TMPDIR:-/tmp}/station-postgres-XXXXXX")"
docker compose exec -T postgres psql -U postgres -d station_db -c \
  'SELECT count(*) FROM stations_latest;'
docker compose exec -T postgres pg_dump -U postgres -Fc station_db > "$backup" &&
  docker compose exec -T postgres pg_restore --list < "$backup" >/dev/null
```

ダンプの検証が成功したら、空の Alpine 側ボリュームで起動して復元します。

```bash
docker compose up -d --build --wait postgres
docker compose exec -T postgres pg_restore --exit-on-error --single-transaction \
  -U postgres -d station_db < "$backup"
docker compose exec -T postgres psql -U postgres -d station_db -c \
  'SELECT count(*) FROM stations_latest;'
```

移行前と駅数を照合し、アプリからの読み出しを確認してから書き込みを再開します。
元の Compose 定義と Debian 側ボリューム、ダンプは復旧用に保持します。
この手順はリポジトリ標準の `station_db` が対象です。

### API と ops の起動

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
