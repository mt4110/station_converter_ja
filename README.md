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
- dev 補助モード: `station-crawler --loop`

本番では `station-crawler` を常駐させません。
本番 ingest の公式入口は **`station-ops job ingest-n02`** です。

## 最短セットアップ

### 1. DB を選ぶ

- PostgreSQL で始める: `postgres`
- MySQL で始める: `mysql`
- SQLite で軽く試す: `sqlite`（ローカル確認向け。Docker は起動しません）

本番の primary write DB は PostgreSQL / MySQL を前提にします。以下は PostgreSQL 例です。

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

`postal_converter_ja` とポート帯をずらしてあるので、同時に立ち上げても衝突しにくいです。

## クイックスタート TUI

まず TUI から触るなら:

```bash
python3 launcher/quickstart_tui.py
```

`Quick Start` で env 準備、migrate、ingest、validate-ingest、DB Web、API、Sample Web をまとめて立ち上げられます。
各項目は個別にも start / stop でき、選択中の項目の直近ログと直近の実行時刻をそのまま見られます。
`Quick Start` 実行中は、右ペインで現在の step と経過時間を見られます。
`v` で validate mode を `standard / strict` に切り替えられ、`x` で実行中の workflow をその場でキャンセルできます。
`l` で英語 / 日本語を切り替えられ、右ペインに「最短で進める順番」の Tips も常に出ます。

### 6. Frontend 例を触る

```bash
cd frontend
npm install
npm run dev
```

sample web は全国駅データを ingest してから使う前提です。

DB をブラウザで見たいときは、TUI の `DB Web UI` を使うか、次を実行します。

```bash
docker compose --profile dbweb up -d adminer
```

## validate-ingest

ingest 後の acceptance check は次で回せます。

```bash
cargo run -p station-ops -- validate-ingest
```

JSON が欲しいときは `--json`、warning も failure 扱いにしたいときは `--strict` を付けてください。
標準の acceptance floor は `active_station_count >= 10000`,
`distinct_station_name_count >= 9000`, `distinct_line_count >= 600`,
`distinct_operator_count >= 170` を基準にしています。

## Ingest Baseline

2026-04-19 時点のローカル PostgreSQL 実測:

- source: MLIT `N02-24`
- parsed_features: `10235`
- parsed_stations: `10155`
- `validate-ingest`: `ok`
- fresh PostgreSQL initial ingest:
  - `load_ms=582`
  - `save_zip_ms=1`
  - `extract_ms=21`
  - `parse_ms=308`
  - `diff_ms=28`
  - `persist_ms=630`
  - `total_ms=2039`
- same snapshot re-ingest with skip:
  - `persist_ms=4`
  - `total_ms=952`

同日の比較で、bulk persistence 導入前の fresh PostgreSQL initial ingest は `persist_ms=10091`, `total_ms=12044` でした。
つまり、支配的だった persist が大きく落ち、最適化の主目的は達成できています。

同じ local ZIP を使った chunk size sweep:

| write chunk | persist_ms | total_ms |
| --- | ---: | ---: |
| `200` | `658` | `1498` |
| `500` | `626` | `1460` |
| `1000` | `596` | `1422` |

この比較では `1000` が最良でした。
ただし SQLite は build によって bind parameter 上限が `999` のことがあるため、実運用の default は SQLite だけ `write=76` / `close=998` に抑え、PostgreSQL は `1000` を維持しています。

2026-04-19 時点のローカル MySQL 実測:

| write chunk | persist_ms | total_ms |
| --- | ---: | ---: |
| `200` | `574` | `1384` |
| `500` | `627` | `1455` |
| `1000` | `632` | `1465` |

この比較では MySQL は `200` が最良でした。
そのため env 未指定時の default は **PostgreSQL = `write=1000` / `close=1000`、MySQL = `write=200` / `close=1000`、SQLite = `write=76` / `close=998`** にしています。

MySQL では default collation が text distinct を 1 件つぶすことがあったため、`validate-ingest` の distinct station / line / operator count は bytewise semantics に寄せて cross-DB で揃えています。
同じ理由で API の `stations/search` / `lines/{line_name}/stations` / `operators/{operator_name}/stations` も MySQL では binary collation を使い、`の` と `ノ` のような別名が混ざらないようにしています。
いずれの数値も local machine の reference 値であり、別マシンや別ストレージ条件での最適値を保証するものではありません。

## Example frontend

- `/examples/station-search` - 駅名検索
- `/examples/address-search` - 住所 / 市区町村から近い駅候補
- `/examples/line-search` - 路線から駅一覧
- `/examples/operator-search` - 事業者から駅一覧
- `/examples/nearby-search` - 緯度経度から近くの駅

`/examples/address-search` は frontend の Next.js route `/api/address-search` を使います。
これは国土地理院 Address Search を包む example helper で、
`station-api` の OpenAPI / generated station SDK には含めていません。

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

`/opt/station_converter_ja/target/release/` に置く binary は次で揃えられます。

```bash
sudo ./scripts/install_release_binaries.sh /opt/station_converter_ja station-converter-ja station-converter-ja
```

セットアップ手順と運用 runbook は [`docs/OPERATIONS.md`](docs/OPERATIONS.md) を参照してください。

## Verify / Release

静的チェックと DB 実経路の確認:

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
cd frontend && npm ci && npm run build
```

`./scripts/verify_repo.sh` は Rust workspace の verify に加えて、
OpenAPI から再生成される frontend の station SDK / 型定義が最新かも確認します。

SQLite export 後の primary DB / artifact parity check:

```bash
cargo run -p station-ops -- verify-sqlite-parity
./scripts/verify_sqlite_reproducibility.sh
```

SQLite artifact を配布物として固める:

```bash
./scripts/release_sqlite_artifact.sh postgres
```

GitHub Release までまとめて公開する:

```bash
./scripts/publish_sqlite_release.sh postgres v0.1.4
```

配布物は `artifacts/sqlite/` に出力されます。公開済み tag は載せ替えず、
次の patch tag で進めます。上の例は、最新 tag が `v0.1.3` の状態から
`v0.1.4` を切る想定です。

公開済み release を利用者側で検証する最短経路:

```bash
REPO=mt4110/station_converter_ja
TAG=v0.1.4
mkdir -p "tmp/release-${TAG}"
gh release download "$TAG" -R "$REPO" -D "tmp/release-${TAG}" --clobber \
  -p stations.sqlite3 \
  -p manifest.json \
  -p SOURCE_METADATA.json \
  -p checksums.txt \
  -p CHANGELOG.md \
  -p RELEASE_NOTES.md \
  -p README_SQLITE.md \
  -p SBOM.spdx.json
cd "tmp/release-${TAG}"
shasum -a 256 -c checksums.txt
gh attestation verify stations.sqlite3 -R "$REPO"
gh attestation verify stations.sqlite3 -R "$REPO" \
  --predicate-type https://spdx.dev/Document/v2.3
```

この artifact の freshness は latest available MLIT N02 snapshot までであり、
real-time railway data ではありません。

## Docs

- [AGENTS.md](AGENTS.md)
  - contributor / automation rules
  - data / release / API change policy
- [CONTRIBUTING.md](CONTRIBUTING.md)
  - local workflow
  - verify checklist
- [docs/OPERATIONS.md](docs/OPERATIONS.md)
  - production runbook
  - systemd 導線
  - 更新時 / 障害時の扱い
- [docs/INDEX.md](docs/INDEX.md)
  - 役割別 docs 入口
- [docs/QUICKSTART_API.md](docs/QUICKSTART_API.md)
  - API local quickstart
- [docs/QUICKSTART_SQLITE.md](docs/QUICKSTART_SQLITE.md)
  - SQLite artifact quickstart
- [docs/DATABASE.md](docs/DATABASE.md)
  - table 構造
  - 軽い sample dump
  - example SQL
- [docs/API.md](docs/API.md)
  - API endpoint overview
  - error envelope
- [docs/RELEASE.md](docs/RELEASE.md)
  - artifact / release 手順
  - verify scripts
- [docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md)
  - tag / publish 前の checklist
- [docs/ARTIFACTS.md](docs/ARTIFACTS.md)
  - SQLite release bundle の中身
  - checksum / attestation verify
- [docs/DATA_FRESHNESS.md](docs/DATA_FRESHNESS.md)
  - source refresh
  - freshness claim
- [docs/DATA_QUALITY.md](docs/DATA_QUALITY.md)
  - validation floor
  - parity checks
- [docs/OPENAPI.md](docs/OPENAPI.md)
  - OpenAPI current state
  - contract polish の残タスク
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
  - runtime 責務分離
  - lock 方針
- [docs/DEPLOY.md](docs/DEPLOY.md)
  - self-hosted / cloud skeleton の位置づけ
- [docs/SOURCE_POLICY.md](docs/SOURCE_POLICY.md)
  - canonical source と license 境界
  - N05 overlay の扱い
- [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md)
  - operator checks
- [docs/REDIS_CACHE.md](docs/REDIS_CACHE.md)
  - cache-only policy
- [docs/FAQ.md](docs/FAQ.md)
  - よくある質問
- [docs/ROADMAP.md](docs/ROADMAP.md)
  - 残タスクの優先順位
  - いまやらないこと
  - milestone 整理

## I want to...

- SQLite artifact を配布 / 検証したい: [docs/RELEASE.md](docs/RELEASE.md), [docs/ROADMAP.md](docs/ROADMAP.md)
- SQLite artifact をローカルで使いたい: [docs/QUICKSTART_SQLITE.md](docs/QUICKSTART_SQLITE.md)
- release bundle の中身を確認したい: [docs/ARTIFACTS.md](docs/ARTIFACTS.md)
- API をローカルで立ち上げたい: [docs/QUICKSTART_API.md](docs/QUICKSTART_API.md), [API_SPEC.md](API_SPEC.md), [docs/OPERATIONS.md](docs/OPERATIONS.md)
- OpenAPI / API contract の現状と残りの polish を見たい: [docs/OPENAPI.md](docs/OPENAPI.md), [API_SPEC.md](API_SPEC.md)
- source freshness を確認したい: [docs/DATA_FRESHNESS.md](docs/DATA_FRESHNESS.md)
- data quality gate を確認したい: [docs/DATA_QUALITY.md](docs/DATA_QUALITY.md)
- self-host したい: [docs/OPERATIONS.md](docs/OPERATIONS.md), [docs/DEPLOY.md](docs/DEPLOY.md)
- DB schema と example SQL を見たい: [docs/DATABASE.md](docs/DATABASE.md)
- source / license 方針を確認したい: [docs/SOURCE_POLICY.md](docs/SOURCE_POLICY.md), [docs/ROADMAP.md](docs/ROADMAP.md)
- 次に詰める残タスクを見たい: [docs/ROADMAP.md](docs/ROADMAP.md)

## データ方針

- primary write DB は **PostgreSQL / MySQL**
- **SQLite は read-only artifact**
- source of truth は **`station_versions`**
- `stations_latest` は view / projection
- `latitude` / `longitude` は代表点
- raw geometry は `geometry_geojson` に保持

現状の crawler は、国土数値情報 N02 の公式 ZIP に同梱される UTF-8 `Station.geojson` を読み込み、
`source_snapshots` / `station_versions` / `station_change_events` まで反映します。

`stations/search` / `lines/{line_name}/stations` / `operators/{operator_name}/stations` は cross-DB semantics を揃えており、MySQL でも `江の島線` と `江ノ島線`、`の` と `ノ` のような別値が混ざらないようにしています。

つまり、この repo はもう空箱ではありません。
N02 one-shot ingest、`created / updated / removed` の初期差分反映、SQLite artifact export、
API、source refresh、品質ゲート、運用導線まで揃った v1 の実働基盤です。ここから overlay、release promotion、cloud deploy を積み増していく前提です。

## いま含めているもの / まだ含めていないもの

含めているもの:

- N02 one-shot ingest
- `created / updated / removed` の初期差分反映
- PostgreSQL / MySQL / SQLite artifact 対応
- product-grade SQLite release bundle
- OpenAPI JSON / Swagger UI / TypeScript SDK generation
- dataset snapshot / change history API
- standard error envelope with optional machine-readable detail
- source freshness watcher
- data quality gates and SQLite parity checks
- example frontend
- self-hosted systemd 導線

まだ含めていないもの:

- N05 overlay parser
- fully automated release promotion after source refresh
- cloud resource の本実装

## License

この repo は **MIT OR Apache-2.0** の dual license です。
利用者は [`LICENSE-MIT`](LICENSE-MIT) または [`LICENSE-APACHE`](LICENSE-APACHE) を選べます。
