#!/usr/bin/env bash
set -euo pipefail

DB_TYPE="${1:-${DATABASE_TYPE:-postgres}}"

case "$DB_TYPE" in
  postgres|mysql)
    ;;
  *)
    echo "usage: $0 [postgres|mysql]" >&2
    exit 1
    ;;
esac

export DATABASE_TYPE="$DB_TYPE"

SQLITE_URL="${SQLITE_DATABASE_URL:-sqlite://storage/sqlite/stations.sqlite3}"
SQLITE_URL_NO_FRAGMENT="${SQLITE_URL%%#*}"

case "$SQLITE_URL_NO_FRAGMENT" in
  sqlite://*)
    SQLITE_TARGET="${SQLITE_URL_NO_FRAGMENT#sqlite://}"
    ;;
  sqlite:*)
    SQLITE_TARGET="${SQLITE_URL_NO_FRAGMENT#sqlite:}"
    ;;
  *)
    echo "unsupported SQLITE_DATABASE_URL: ${SQLITE_URL}" >&2
    exit 1
    ;;
esac

SQLITE_QUERY=""
case "$SQLITE_TARGET" in
  *\?*)
    SQLITE_QUERY="${SQLITE_TARGET#*\?}"
    SQLITE_PATH="${SQLITE_TARGET%%\?*}"
    ;;
  *)
    SQLITE_PATH="$SQLITE_TARGET"
    ;;
esac

case "$SQLITE_PATH" in
  :memory:|"")
    echo "SQLITE_DATABASE_URL must point to a file, not an in-memory database" >&2
    exit 1
    ;;
esac

case "&${SQLITE_QUERY}&" in
  *"&mode=memory&"*)
    echo "SQLITE_DATABASE_URL must point to a file, not an in-memory database" >&2
    exit 1
    ;;
esac

echo "exporting SQLite artifact from ${DB_TYPE}"
cargo run -p station-ops -- export-sqlite

echo "packaging SQLite artifact from ${SQLITE_PATH}"
"$(cd "$(dirname "$0")" && pwd)/package_sqlite_release.sh" "$SQLITE_PATH"

echo "release artifact ready under artifacts/sqlite"
