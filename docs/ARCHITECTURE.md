# ARCHITECTURE

## Runtime pieces

- `worker/crawler`
  - N02 one-shot ingest の実行核
  - dev 用 loop 実行
  - snapshot download / normalize / diff event generation
- `worker/api`
  - station search API
  - line search API
  - nearby search API
- `worker/ops`
  - migrations
  - one-shot job runner
  - export / packaging
- `frontend`
  - sample integration UI
- `infra/terraform`
  - cloud deployment skeleton
- `deploy`
  - self-hosted `systemd` 実ファイル
  - future k8s / helm / argocd path

## Data model

```text
source_snapshots -> station_versions -> stations_latest(view)
                  -> station_change_events
station_identities -> station_versions
```

## Non-goals in v1 scaffold

- full GIS stack
- PostGIS-first schema
- direct SQLite write path from crawler
- all clouds production-ready on day one

## Runtime policy

- production 標準運用は `station-ops job ingest-n02`
- production scheduler は external scheduler / systemd timer を前提
- `station-crawler -- --loop` は dev 補助モード
- lock は `JOB_LOCK_DIR` 配下の file lock で二重実行を防ぐ
- repo root から起動する Rust コマンドは `worker/*/.env` を自動読込する

運用の一本化された説明は [`docs/OPERATIONS.md`](./OPERATIONS.md) を参照してください。
