# station_converter_ja

High-performance Japanese Station Converter & API. Auto-updating, diff-aware, multi-DB delivery (PostgreSQL / MySQL / SQLite artifact), written in Rust & Next.js.

全国の駅データを自動取得・差分管理し、駅検索 / 路線検索 / 近傍検索 API を提供するための雛形です。  
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

`postal_converter_ja` とポート帯をずらしてあるので、同時に立ち上げても衝突しにくいです。

## クイックスタート

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

### 5. API

```bash
cargo run -p station-api
```

### 6. Crawler

```bash
cargo run -p station-crawler -- --once
```

### 7. Frontend

```bash
cd frontend
npm install
npm run dev
```

## Example frontend

`postal_converter_ja` と同じノリで、導入サンプルを置く前提です。

- `/examples/station-search` — 駅名検索
- `/examples/line-search` — 路線から駅一覧
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
