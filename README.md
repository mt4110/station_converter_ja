# station_converter_ja

High-performance Japanese Station Converter & API. Auto-updating, diff-aware, multi-DB delivery (PostgreSQL / MySQL / SQLite artifact), written in Rust & Next.js.

全国の駅データを自動取得・差分管理し、駅検索 / 路線検索 / 事業者検索 / 近傍検索 API を提供するための雛形です。  
`postal_converter_ja` と同じ思想で、**Rust の crawler + API**、**Next.js の example frontend**、**Nix flakes + Docker / Collima**、**Terraform 連携**を前提にしています。

## 方針

- primary write DB は **PostgreSQL / MySQL**
- **SQLite は read-only artifact / 配布用途**
- Redis は **任意キャッシュ**
- source of truth は **station_versions**（immutable）
- `stations_latest` は view / projection
- 駅の `latitude` / `longitude` は **代表点**。原本 geometry は `geometry_geojson` に保持
- MISE は使わず、**Nix flakes** を使う
- **NixOS 専用 repo にはしない**。NixOS / macOS / Linux で同じ repo を使う

## リポジトリ説明文（GitHub Description）

> High-performance Japanese Station Converter & API. Auto-updating, diff-aware, multi-DB delivery (PostgreSQL / MySQL / SQLite artifact), written in Rust & Next.js.

日本語寄せにするなら:

> 全国駅データを自動更新・差分管理する Rust 製 Station Converter & API。PostgreSQL / MySQL / SQLite artifact 対応、Next.js example 付き。

## 想定ソース

- v1 canonical: 国土数値情報 `N02`
- optional overlay: `N05`（**非商用制限あり**のため opt-in）
- future overlay: 駅データ.jp など

現状の crawler は、**国土数値情報 N02 の公式 ZIP に同梱されている UTF-8 `Station.geojson`** を読み込み、
`source_snapshots` / `station_versions` / `station_change_events` まで反映します。  
同一駅レコード内の分割線形は自動で束ね、代表点を `latitude` / `longitude` に計算して保持します。

## 主要ディレクトリ

```text
station_converter_ja/
  .github/
    workflows/
  artifacts/
    sqlite/
  deploy/
    helm/station-converter-ja/
    k8s/base/
    argocd/
  docs/
  frontend/
  infra/
    terraform/
      modules/
      platforms/aws/
      platforms/gcp/
      platforms/azure/
  launcher/
  scripts/
  storage/
    migrations/
      postgres/
      mysql/
      sqlite/
    schema/
    sqlite/
  worker/
    shared/
    api/
    crawler/
    ops/
```

## ローカルポート

- API: `3212`
- Frontend: `3213`
- MySQL: `3214`
- PostgreSQL: `3215`
- Redis: `3216`
- DB Web (Adminer): `3217`

`postal_converter_ja` とポート帯をずらしてあるので、同時に立ち上げても衝突しにくいです。

## クイックスタート

まず TUI から触るなら:

```bash
python3 launcher/quickstart_tui.py
```

`Quick Start` で env 準備、migrate、ingest、validate-ingest、DB Web、API、Sample Web をまとめて立ち上げられます。  
各項目は個別にも start / stop でき、選択中の項目の直近ログと直近の実行時刻をそのまま見られます。  
`Quick Start` 実行中は、右ペインで現在の step と経過時間を見られます。  
`v` で validate mode を `standard / strict` に切り替えられ、`x` で実行中の workflow をその場でキャンセルできます。  
`l` で英語 / 日本語を切り替えられ、右ペインに「最短で進める順番」の Tips も常に出ます。

### 1. DB 起動

```bash
docker compose up -d
docker compose --profile cache up -d redis
```

macOS + Collima の例:

```bash
colima start
docker context use colima
docker compose up -d
```

### 2. env 配置

```bash
cp worker/api/.env.example worker/api/.env
cp worker/crawler/.env.example worker/crawler/.env
cp worker/ops/.env.example worker/ops/.env
cp frontend/.env.local.example frontend/.env.local
```

### 3. Nix shell

```bash
nix develop
```

### 4. migrate

```bash
cargo run -p station-ops -- migrate
```

### 5. Ingest

```bash
cargo run -p station-ops -- job ingest-n02
```

PostgreSQL / MySQL で将来の artifact 導線まで同じ入口に寄せたい場合:

```bash
cargo run -p station-ops -- job ingest-n02 --export-sqlite
```

現時点では `--export-sqlite` は受け付けますが、SQLite export 自体はまだ scaffold 段階です。

ローカルの fixture / ZIP を意図的に ingest したいときだけ、`ALLOW_LOCAL_SOURCE_SNAPSHOT=true` を付けてください。

### 6. validate-ingest

```bash
cargo run -p station-ops -- validate-ingest
```

JSON が欲しいときは `--json`、warning も failure 扱いにしたいときは `--strict` を付けてください。

### 7. API

```bash
cargo run -p station-api
```

bulk persistence の chunk size は env で比較できます。

```bash
INGEST_WRITE_CHUNK_SIZE=1000
INGEST_CLOSE_CHUNK_SIZE=1000
```

`INGEST_WRITE_CHUNK_SIZE` は identity / version / change_event の batch size、`INGEST_CLOSE_CHUNK_SIZE` は stale version close update の chunk size です。
env を省略したときの default は DB ごとに分かれており、現在は PostgreSQL / SQLite が `1000`、MySQL が `200` です。

### 8. Frontend

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

この比較では `1000` が最良だったため、PostgreSQL / SQLite の default は `1000` です。

2026-04-19 時点のローカル MySQL 実測:

- same local ZIP + fresh MySQL initial ingest:

| write chunk | persist_ms | total_ms |
| --- | ---: | ---: |
| `200` | `574` | `1384` |
| `500` | `627` | `1455` |
| `1000` | `632` | `1465` |

この比較では MySQL は `200` が最良でした。  
そのため env 未指定時の default は **PostgreSQL / SQLite = `1000`、MySQL = `200`** にしています。

MySQL では default collation が text distinct を 1 件つぶすことがあったため、`validate-ingest` の distinct line / operator count は bytewise semantics に寄せて cross-DB で揃えています。  
同じ理由で API の `stations/search` / `lines/{line_name}/stations` / `operators/{operator_name}/stations` も MySQL では binary collation を使い、`の` と `ノ` のような別名が混ざらないようにしています。  
いずれの数値も local machine の reference 値であり、別マシンや別ストレージ条件での最適値を保証するものではありません。

## Example frontend

`postal_converter_ja` と同じノリで、導入サンプルを置く前提です。

- `/examples/station-search` — 駅名検索
- `/examples/address-search` — 住所 / 市区町村から近い駅候補
- `/examples/line-search` — 路線から駅一覧
- `/examples/operator-search` — 事業者から駅一覧
- `/examples/nearby-search` — 緯度経度から近くの駅

## `.github` 方針

コミュニティヘルス系は `mt4110/.github` に寄せ、**repo local には workflow だけ置く**前提です。  
この repo には `.github/workflows/` だけ置いています。

## まだやっていないこと

この雛形は **repo bootstrap** です。  
やっていないもの:

- N05 overlay parser
- SQLite export 本実装
- Terraform の本番 resource 実装
- production-ready OpenAPI

ただし、v1 の土台として必要な **N02 station ingest と created / updated / removed の初期差分反映** は入っています。  
つまり、**もう空箱ではない。ここから機能を増やしていける状態**です。
