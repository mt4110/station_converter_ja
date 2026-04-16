# station_converter_ja

全国駅データを自動更新・差分管理する高性能な変換ツール＆APIです。  
PostgreSQL / MySQL を primary write DB にし、SQLite を read-only artifact として配布できます。  
国土数値情報 N02 を取り込み、差分を version / change event として保持しつつ、API と SQLite artifact を提供します。

English README: [README_EN.md](README_EN.md)

## まず最初に押さえること

この repo は `postal_converter_ja` と同じく、役割をはっきり分けています。

- 常駐させるもの: `station-api`
- scheduler から叩くもの: `station-ops job ingest-n02`
- artifact までまとめる補助導線: `station-ops job ingest-n02 --export-sqlite`
- dev 補助モード: `station-crawler -- --loop`

本番では `station-crawler` を常駐させません。  
本番 ingest の公式入口は **`station-ops job ingest-n02`** です。

## 最短セットアップ

### 1. DB を選ぶ

- PostgreSQL で始める: `postgres`
- MySQL で始める: `mysql`

以下は PostgreSQL 例です。

### 2. env を作って DB を起動する

```bash
./scripts/setup_nix_docker.sh postgres
```

これは次をまとめて行います。

- `worker/api/.env`
- `worker/crawler/.env`
- `worker/ops/.env`
- `frontend/.env.local`
- `storage/locks`
- `storage/sqlite`
- `worker/crawler/temp_assets`
- `docker compose up -d postgres`

Rust 側のコマンドは、repo root から起動すると `worker/*/.env` を自動読込します。

### 3. 開発 shell に入る

```bash
nix develop
```

### 4. migrate -> ingest -> export を通す

```bash
cargo run -p station-ops -- migrate
cargo run -p station-ops -- job ingest-n02 --export-sqlite
```

ここまでで次が揃います。

- primary write DB に N02 データを反映
- `storage/sqlite/stations.sqlite3` を生成

### 5. API を起動する

```bash
cargo run -p station-api
```

既定ポート:

- API: `3212`
- Frontend: `3213`
- MySQL: `3214`
- PostgreSQL: `3215`
- Redis: `3216`

### 6. Frontend 例を触る

```bash
cd frontend
npm install
npm run dev
```

## 本番運用の導線

### Self-hosted の標準形

- resident service: `station-api`
- scheduled job: `station-ops job ingest-n02`
- optional chained job: `station-ops job ingest-n02 --export-sqlite`

`external scheduler` でも `systemd timer` でも、この one-shot job を呼ぶ形に揃えます。

### systemd を使う場合

実ファイルを [`deploy/systemd/`](deploy/systemd/) に置いてあります。

- `station-converter-ja-api.service`
- `station-converter-ja-ingest-n02.service`
- `station-converter-ja-ingest-n02.timer`
- `station-converter-ja.env.example`

セットアップ手順と運用 runbook は [`docs/OPERATIONS.md`](docs/OPERATIONS.md) を参照してください。

## Verify / Release

静的チェックと DB 実経路の確認:

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
```

SQLite artifact を配布物として固める:

```bash
./scripts/release_sqlite_artifact.sh postgres
```

配布物は `artifacts/sqlite/` に出力されます。

## Docs

- [docs/OPERATIONS.md](docs/OPERATIONS.md)
  - production runbook
  - systemd 導線
  - 更新時 / 障害時の扱い
- [docs/RELEASE.md](docs/RELEASE.md)
  - artifact / release 手順
  - verify scripts
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
  - runtime 責務分離
  - lock 方針
- [docs/DEPLOY.md](docs/DEPLOY.md)
  - self-hosted / cloud skeleton の位置づけ

## データ方針

- primary write DB は **PostgreSQL / MySQL**
- **SQLite は read-only artifact**
- source of truth は **`station_versions`**
- `stations_latest` は view / projection
- `latitude` / `longitude` は代表点
- raw geometry は `geometry_geojson` に保持

現状の crawler は、国土数値情報 N02 の公式 ZIP に同梱される UTF-8 `Station.geojson` を読み込み、
`source_snapshots` / `station_versions` / `station_change_events` まで反映します。

つまり、この repo はもう空箱ではありません。  
N02 one-shot ingest、`created / updated / removed` の初期差分反映、SQLite artifact export、
API、運用導線まで揃った v1 の実働基盤です。ここから overlay、OpenAPI、cloud deploy を積み増していく前提です。

## いま含めているもの / まだ含めていないもの

含めているもの:

- N02 one-shot ingest
- `created / updated / removed` の初期差分反映
- PostgreSQL / MySQL / SQLite artifact 対応
- example frontend
- self-hosted systemd 導線

まだ含めていないもの:

- N05 overlay parser
- production-ready OpenAPI
- cloud resource の本実装
