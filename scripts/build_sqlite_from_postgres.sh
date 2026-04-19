#!/usr/bin/env bash
set -euo pipefail

export DATABASE_TYPE="${DATABASE_TYPE:-postgres}"

if [ "${DATABASE_TYPE}" != "postgres" ]; then
  echo "error: scripts/build_sqlite_from_postgres.sh only supports DATABASE_TYPE=postgres, got '${DATABASE_TYPE}'" >&2
  echo "hint: use ./scripts/release_sqlite_artifact.sh mysql for the MySQL release flow" >&2
  exit 1
fi

echo "building SQLite artifact from ${DATABASE_TYPE}"
cargo run -p station-ops -- export-sqlite

echo "SQLite artifact ready at ${SQLITE_DATABASE_URL:-sqlite://storage/sqlite/stations.sqlite3}"
echo "for a distributable bundle, run: ./scripts/release_sqlite_artifact.sh ${DATABASE_TYPE}"
