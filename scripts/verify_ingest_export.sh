#!/usr/bin/env bash
set -euo pipefail

DB_TYPE="${1:-postgres}"

case "$DB_TYPE" in
  postgres|mysql)
    ;;
  *)
    echo "usage: $0 [postgres|mysql]" >&2
    exit 1
    ;;
esac

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
FIXTURE_ROOT="$ROOT_DIR/testdata/n02"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/station-verify-${DB_TYPE}-XXXXXX")"
SNAPSHOT_ZIP="$RUN_DIR/N02-24_GML.zip"

cleanup() {
  rm -rf "$RUN_DIR"
}
trap cleanup EXIT

python3 - "$FIXTURE_ROOT" "$SNAPSHOT_ZIP" <<'PY'
import pathlib
import sys
import zipfile

fixture_root = pathlib.Path(sys.argv[1])
snapshot_zip = pathlib.Path(sys.argv[2])

with zipfile.ZipFile(snapshot_zip, "w", compression=zipfile.ZIP_DEFLATED) as archive:
    for path in fixture_root.rglob("*"):
        if path.is_file():
            archive.write(path, path.relative_to(fixture_root))
PY

SNAPSHOT_SHA="$(python3 - "$SNAPSHOT_ZIP" <<'PY'
import hashlib
import pathlib
import sys

snapshot_zip = pathlib.Path(sys.argv[1])
print(hashlib.sha256(snapshot_zip.read_bytes()).hexdigest())
PY
)"

export DATABASE_TYPE="$DB_TYPE"
export SOURCE_SNAPSHOT_URL="file://$SNAPSHOT_ZIP"
export ALLOW_LOCAL_SOURCE_SNAPSHOT=true
export TEMP_ASSET_DIR="$RUN_DIR/temp_assets"
export JOB_LOCK_DIR="$RUN_DIR/locks"
export SQLITE_DATABASE_URL="sqlite://$RUN_DIR/stations.sqlite3"

case "$DB_TYPE" in
  postgres)
    export POSTGRES_DATABASE_URL="${POSTGRES_DATABASE_URL:-postgres://postgres:postgres_password@127.0.0.1:3215/station_db}"
    ;;
  mysql)
    export MYSQL_DATABASE_URL="${MYSQL_DATABASE_URL:-mysql://station_user:station_password@127.0.0.1:3214/station_db}"
    ;;
esac

echo "verifying migrate -> ingest -> export on ${DB_TYPE}"

(
  cd "$ROOT_DIR"
  cargo run -p station-ops -- migrate
  cargo run -p station-ops -- reset-verify-db --yes
  cargo run -p station-ops -- job ingest-n02 --export-sqlite
)

python3 - "$RUN_DIR/stations.sqlite3" "$SNAPSHOT_SHA" <<'PY'
import sqlite3
import sys

db_path = sys.argv[1]
snapshot_sha = sys.argv[2]

conn = sqlite3.connect(db_path)
try:
    snapshot_row = conn.execute(
        """
        SELECT
            ss.id,
            COUNT(DISTINCT sv.id) AS station_versions,
            COUNT(DISTINCT sce.id) AS station_change_events
        FROM source_snapshots AS ss
        LEFT JOIN station_versions AS sv
          ON sv.snapshot_id = ss.id
        LEFT JOIN station_change_events AS sce
          ON sce.snapshot_id = ss.id
        WHERE ss.source_sha256 = ?
        GROUP BY ss.id
        ORDER BY station_versions DESC, station_change_events DESC, ss.id DESC
        LIMIT 1
        """,
        (snapshot_sha,),
    ).fetchone()
    if snapshot_row is None:
        raise SystemExit(f"fixture snapshot not exported: {snapshot_sha}")

    snapshot_id, snapshot_versions, snapshot_change_events = snapshot_row
    if snapshot_versions != 2:
        raise SystemExit(
            f"station_versions for fixture snapshot mismatch: expected 2, got {snapshot_versions}"
        )

    if snapshot_change_events < 2:
        raise SystemExit(
            "station_change_events for fixture snapshot should be at least 2, "
            f"got {snapshot_change_events}"
        )

    latest_fixture_stations = conn.execute(
        """
        SELECT COUNT(*)
        FROM stations_latest
        WHERE (station_name, line_name, operator_name) IN (
            ('新宿', '京王線', '京王電鉄'),
            ('中野', '中央線', '東日本旅客鉄道')
        )
        """
    ).fetchone()[0]
    if latest_fixture_stations != 2:
        raise SystemExit(
            f"latest fixture station count mismatch: expected 2, got {latest_fixture_stations}"
        )
finally:
    conn.close()

print(
    "verified exported fixture snapshot:",
    {
        "snapshot_id": snapshot_id,
        "station_versions": snapshot_versions,
        "station_change_events": snapshot_change_events,
        "latest_fixture_stations": latest_fixture_stations,
    },
)
PY

echo "verification complete for ${DB_TYPE}"
