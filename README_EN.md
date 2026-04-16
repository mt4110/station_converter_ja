# station_converter_ja

Rust-based Japanese Station Converter & API for automated nationwide station updates, diff tracking, and SQLite artifact delivery.

Japanese README: [README.md](README.md)

## Start Here

This repository keeps runtime roles intentionally separate.

- long-running service: `station-api`
- scheduled production job: `station-ops job ingest-n02`
- optional chained artifact flow: `station-ops job ingest-n02 --export-sqlite`
- dev helper loop: `station-crawler -- --loop`

Production should not keep `station-crawler` running as a resident worker.  
The official ingest entry point is **`station-ops job ingest-n02`**.

## Fastest Setup

### 1. Pick a primary database

- `postgres`
- `mysql`

The example below uses PostgreSQL.

### 2. Create env files and start the database

```bash
./scripts/setup_nix_docker.sh postgres
```

This prepares:

- `worker/api/.env`
- `worker/crawler/.env`
- `worker/ops/.env`
- `frontend/.env.local`
- `storage/locks`
- `storage/sqlite`
- `worker/crawler/temp_assets`
- `docker compose up -d postgres`

Rust commands auto-load `worker/*/.env` when launched from the repository root.

### 3. Enter the dev shell

```bash
nix develop
```

### 4. Run migrate -> ingest -> export

```bash
cargo run -p station-ops -- migrate
cargo run -p station-ops -- job ingest-n02 --export-sqlite
```

At this point you have:

- N02 data in your primary write database
- `storage/sqlite/stations.sqlite3` rebuilt as the read-only artifact

### 5. Start the API

```bash
cargo run -p station-api
```

Default ports:

- API: `3212`
- Frontend: `3213`
- MySQL: `3214`
- PostgreSQL: `3215`
- Redis: `3216`

### 6. Try the example frontend

```bash
cd frontend
npm install
npm run dev
```

## Production Flow

### Standard self-hosted shape

- resident service: `station-api`
- scheduled job: `station-ops job ingest-n02`
- optional chained job: `station-ops job ingest-n02 --export-sqlite`

Whether you use an external scheduler or `systemd timer`, the contract stays the same: call the one-shot job.

### systemd path

Ready-to-use reference files live in [`deploy/systemd/`](deploy/systemd/).

- `station-converter-ja-api.service`
- `station-converter-ja-ingest-n02.service`
- `station-converter-ja-ingest-n02.timer`
- `station-converter-ja.env.example`

See [`docs/OPERATIONS.md`](docs/OPERATIONS.md) for install and runbook details.

## Verify / Release

Static verification plus database-backed flow checks:

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
```

Build a distributable SQLite bundle:

```bash
./scripts/release_sqlite_artifact.sh postgres
```

Outputs are written to `artifacts/sqlite/`.

## Docs

- [docs/OPERATIONS.md](docs/OPERATIONS.md)
  - production runbook
  - systemd path
  - update and failure handling
- [docs/RELEASE.md](docs/RELEASE.md)
  - artifact / release flow
  - verify scripts
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
  - runtime responsibility split
  - lock policy
- [docs/DEPLOY.md](docs/DEPLOY.md)
  - self-hosted and cloud deployment positioning

## Data Policy

- Primary write DBs are **PostgreSQL / MySQL**
- **SQLite is a read-only artifact**
- The source of truth is **`station_versions`**
- `stations_latest` is a view / projection
- `latitude` / `longitude` are representative points
- raw geometry stays in `geometry_geojson`

The current crawler reads the UTF-8 `Station.geojson` distributed inside the official N02 ZIP and writes through
`source_snapshots`, `station_versions`, and `station_change_events`.

In other words, this repository is no longer an empty scaffold.  
It already has a working v1 foundation: N02 one-shot ingest, initial `created / updated / removed` diff handling,
SQLite artifact export, the API surface, and the operational path. From here, the plan is to extend it with overlays,
OpenAPI, and cloud deployment resources.

## Included vs Not Yet Included

Included:

- N02 one-shot ingest
- initial `created / updated / removed` diff handling
- PostgreSQL / MySQL / SQLite artifact delivery
- example frontend
- self-hosted systemd path

Not yet included:

- N05 overlay parser
- production-ready OpenAPI
- full cloud resource implementations
