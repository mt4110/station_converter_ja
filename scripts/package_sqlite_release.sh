#!/usr/bin/env bash
set -euo pipefail

mkdir -p artifacts/sqlite

DB_PATH="${1:-storage/sqlite/stations.sqlite3}"

if [[ ! -f "$DB_PATH" ]]; then
  echo "SQLite DB not found: $DB_PATH"
  exit 1
fi

STAMP="$(date +%Y%m%d)"
OUT_DB="artifacts/sqlite/stations-${STAMP}.sqlite3"
OUT_SUM="artifacts/sqlite/checksums-${STAMP}.txt"
OUT_MANIFEST="artifacts/sqlite/manifest-${STAMP}.txt"

cp "$DB_PATH" "$OUT_DB"
sha256sum "$OUT_DB" > "$OUT_SUM"

cat > "$OUT_MANIFEST" <<EOF
name=station_converter_ja sqlite artifact
date=${STAMP}
file=$(basename "$OUT_DB")
checksum_file=$(basename "$OUT_SUM")
EOF

echo "packaged:"
echo "  $OUT_DB"
echo "  $OUT_SUM"
echo "  $OUT_MANIFEST"
