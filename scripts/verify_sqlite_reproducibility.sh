#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/station-repro-XXXXXX")"

cleanup() {
  rm -rf "$RUN_DIR"
}
trap cleanup EXIT

export SQLITE_DATABASE_URL="sqlite://$RUN_DIR/one.sqlite3"
(cd "$ROOT_DIR" && cargo run -p station-ops -- export-sqlite >/dev/null)

export SQLITE_DATABASE_URL="sqlite://$RUN_DIR/two.sqlite3"
(cd "$ROOT_DIR" && cargo run -p station-ops -- export-sqlite >/dev/null)

python3 - "$RUN_DIR/one.sqlite3" "$RUN_DIR/two.sqlite3" <<'PY'
import hashlib
import json
import sqlite3
import sys

TABLES = [
    "source_snapshots",
    "station_identities",
    "station_versions",
    "station_change_events",
]


def logical_digest(path):
    digest = hashlib.sha256()
    conn = sqlite3.connect(path)
    conn.row_factory = sqlite3.Row
    try:
        for table in TABLES:
            rows = conn.execute(f"SELECT * FROM {table} ORDER BY id").fetchall()
            payload = {
                "table": table,
                "rows": [dict(row) for row in rows],
            }
            digest.update(json.dumps(payload, ensure_ascii=False, sort_keys=True).encode("utf-8"))
            digest.update(b"\n")
    finally:
        conn.close()

    return digest.hexdigest()


left = logical_digest(sys.argv[1])
right = logical_digest(sys.argv[2])

if left != right:
    raise SystemExit(f"logical sqlite export digest mismatch: {left} != {right}")

print(f"logical sqlite export digest: {left}")
PY

