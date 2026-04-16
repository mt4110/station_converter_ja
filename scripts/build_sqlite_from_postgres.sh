#!/usr/bin/env bash
set -euo pipefail

export DATABASE_TYPE="${DATABASE_TYPE:-postgres}"

echo "building SQLite artifact from ${DATABASE_TYPE}"
cargo run -p station-ops -- export-sqlite

echo "SQLite artifact ready at ${SQLITE_DATABASE_URL:-sqlite://storage/sqlite/stations.sqlite3}"
echo "for a distributable bundle, run: ./scripts/release_sqlite_artifact.sh ${DATABASE_TYPE}"
