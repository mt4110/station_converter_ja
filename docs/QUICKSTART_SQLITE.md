# Quickstart: SQLite Artifact

SQLite is a read-only distribution artifact. It is useful for local inspection, demos,
and consumers that want a portable snapshot without running PostgreSQL or MySQL.

## Build Locally

```bash
./scripts/setup_nix_docker.sh postgres
nix develop
cargo run -p station-ops -- migrate
cargo run -p station-ops -- job ingest-n02 --export-sqlite
```

The artifact is written to `storage/sqlite/stations.sqlite3` by default.

## Verify Parity

After export, compare primary DB counts against the SQLite artifact:

```bash
cargo run -p station-ops -- verify-sqlite-parity
```

The command checks source snapshots, identities, versions, change events,
`stations_latest`, and the latest source snapshot digest.

## Inspect

```bash
sqlite3 storage/sqlite/stations.sqlite3
```

Useful checks:

```sql
SELECT COUNT(*) FROM stations_latest;
SELECT source_version, source_sha256 FROM source_snapshots ORDER BY id DESC LIMIT 1;
SELECT change_kind, COUNT(*) FROM station_change_events GROUP BY change_kind;
```

