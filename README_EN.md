# station_converter_ja

High-performance converter and API for nationwide station data with automated updates, diff tracking, and SQLite artifact delivery.  
It ingests MLIT N02 data, keeps version and change-event history, and serves the dataset through the API and distributable SQLite artifacts.

Japanese README: [README.md](README.md)

## Start Here

This repository keeps runtime roles intentionally separate.

- long-running service: `station-api`
- scheduled production job: `station-ops job ingest-n02`
- optional chained artifact flow: `station-ops job ingest-n02 --export-sqlite`
- dev helper loop: `station-crawler --loop`

Production should not keep `station-crawler` running as a resident worker.  
The official ingest entry point is **`station-ops job ingest-n02`**.

## Fastest Setup

### 1. Pick a primary database

- `postgres`
- `mysql`
- `sqlite` for a lightweight local pass (Docker is skipped)

Production primary write DBs should stay on PostgreSQL / MySQL. The example below uses PostgreSQL.

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

You can build and stage the binaries expected under `/opt/station_converter_ja/target/release/` with:

```bash
sudo ./scripts/install_release_binaries.sh /opt/station_converter_ja station-converter-ja station-converter-ja
```

See [`docs/OPERATIONS.md`](docs/OPERATIONS.md) for install and runbook details.

## Verify / Release

Static verification plus database-backed flow checks:

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
cd frontend && npm ci && npm run build
```

`./scripts/verify_repo.sh` now also runs the frontend station SDK freshness check,
so OpenAPI changes cannot drift away from regenerated client artifacts.

Build a distributable SQLite bundle:

```bash
./scripts/release_sqlite_artifact.sh postgres
```

Publish the SQLite bundle to a GitHub Release:

```bash
./scripts/publish_sqlite_release.sh postgres v0.1.1
```

Outputs are written to `artifacts/sqlite/`.

## Docs

- [AGENTS.md](AGENTS.md)
  - contributor / automation rules
  - data, release, and API change policy
- [CONTRIBUTING.md](CONTRIBUTING.md)
  - local workflow
  - verification checklist
- [docs/OPERATIONS.md](docs/OPERATIONS.md)
  - production runbook
  - systemd path
  - update and failure handling
- [docs/DATABASE.md](docs/DATABASE.md)
  - table layout
  - lightweight sample dump
  - example SQL
- [docs/RELEASE.md](docs/RELEASE.md)
  - artifact / release flow
  - verify scripts
- [docs/ARTIFACTS.md](docs/ARTIFACTS.md)
  - SQLite release bundle contents
  - checksum / attestation verification
- [docs/OPENAPI.md](docs/OPENAPI.md)
  - first-pass OpenAPI design
  - `/openapi.json` and `/docs` plan
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
  - runtime responsibility split
  - lock policy
- [docs/DEPLOY.md](docs/DEPLOY.md)
  - self-hosted and cloud deployment positioning
- [docs/SOURCE_POLICY.md](docs/SOURCE_POLICY.md)
  - canonical source and licensing boundary
  - N05 overlay policy
- [docs/ROADMAP.md](docs/ROADMAP.md)
  - remaining priorities
  - what not to do yet
  - milestone plan

## I want to...

- ship or verify the SQLite artifact: [docs/RELEASE.md](docs/RELEASE.md), [docs/ROADMAP.md](docs/ROADMAP.md)
- inspect the release bundle contents: [docs/ARTIFACTS.md](docs/ARTIFACTS.md)
- run the API locally: [README.md](README.md), [API_SPEC.md](API_SPEC.md), [docs/OPERATIONS.md](docs/OPERATIONS.md)
- review the next API contract plan: [docs/OPENAPI.md](docs/OPENAPI.md), [API_SPEC.md](API_SPEC.md)
- self-host in production: [docs/OPERATIONS.md](docs/OPERATIONS.md), [docs/DEPLOY.md](docs/DEPLOY.md)
- inspect the schema and example SQL: [docs/DATABASE.md](docs/DATABASE.md)
- confirm source and licensing policy: [docs/SOURCE_POLICY.md](docs/SOURCE_POLICY.md), [docs/ROADMAP.md](docs/ROADMAP.md)
- see the next remaining tasks: [docs/ROADMAP.md](docs/ROADMAP.md)

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
- product-grade SQLite release bundle
- example frontend
- self-hosted systemd path

Not yet included:

- N05 overlay parser
- production-ready OpenAPI
- freshness watcher / publish pipeline
- product-grade data quality gates
- full cloud resource implementations

## License

This repository is dual-licensed under **MIT OR Apache-2.0**.  
You may choose either [`LICENSE-MIT`](LICENSE-MIT) or [`LICENSE-APACHE`](LICENSE-APACHE).
