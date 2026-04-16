#!/usr/bin/env bash
set -euo pipefail

mkdir -p artifacts/sqlite

DB_PATH="${1:-storage/sqlite/stations.sqlite3}"

if [[ ! -f "$DB_PATH" ]]; then
  echo "SQLite DB not found: $DB_PATH"
  exit 1
fi

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DB="artifacts/sqlite/stations-${STAMP}.sqlite3"
OUT_SUM="artifacts/sqlite/checksums-${STAMP}.txt"
OUT_MANIFEST="artifacts/sqlite/manifest-${STAMP}.txt"

if command -v sha256sum >/dev/null 2>&1; then
  CHECKSUM_CMD=(sha256sum)
elif command -v shasum >/dev/null 2>&1; then
  CHECKSUM_CMD=(shasum -a 256)
else
  echo "sha256 checksum command not found" >&2
  exit 1
fi

cp "$DB_PATH" "$OUT_DB"
CHECKSUM_LINE="$("${CHECKSUM_CMD[@]}" "$OUT_DB")"
printf '%s\n' "$CHECKSUM_LINE" > "$OUT_SUM"
CHECKSUM_VALUE="$(printf '%s\n' "$CHECKSUM_LINE" | awk '{print $1}')"
SIZE_BYTES="$(wc -c < "$OUT_DB" | tr -d ' ')"

cat > "$OUT_MANIFEST" <<EOF
name=station_converter_ja sqlite artifact
created_at_utc=${STAMP}
file=$(basename "$OUT_DB")
sha256=${CHECKSUM_VALUE}
size_bytes=${SIZE_BYTES}
checksum_file=$(basename "$OUT_SUM")
EOF

echo "packaged:"
echo "  $OUT_DB"
echo "  $OUT_SUM"
echo "  $OUT_MANIFEST"
