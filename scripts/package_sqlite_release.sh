#!/usr/bin/env bash
set -euo pipefail

mkdir -p artifacts/sqlite

DB_PATH="${1:-storage/sqlite/stations.sqlite3}"
RELEASE_VERSION_INPUT="${2:-}"

if [[ ! -f "$DB_PATH" ]]; then
  echo "SQLite DB not found: $DB_PATH" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
STAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

if [[ -n "$RELEASE_VERSION_INPUT" ]]; then
  RELEASE_VERSION="$RELEASE_VERSION_INPUT"
else
  SHORT_SHA="$(git -C "$ROOT_DIR" rev-parse --short HEAD)"
  DIRTY_SUFFIX=""
  if [[ -n "$(git -C "$ROOT_DIR" status --porcelain --untracked-files=no)" ]]; then
    DIRTY_SUFFIX="-dirty"
  fi
  RELEASE_VERSION="dev-${SHORT_SHA}${DIRTY_SUFFIX}"
fi

SAFE_RELEASE_VERSION="$(printf '%s' "$RELEASE_VERSION" | tr '/[:space:]' '--')"
BUNDLE_DIR="$ROOT_DIR/artifacts/sqlite/station_converter_ja-${SAFE_RELEASE_VERSION}-sqlite-$(date -u +%Y%m%dT%H%M%SZ)"

python3 "$ROOT_DIR/scripts/build_release_bundle.py" \
  --repo-root "$ROOT_DIR" \
  --sqlite-path "$DB_PATH" \
  --bundle-dir "$BUNDLE_DIR" \
  --release-version "$RELEASE_VERSION" \
  --generated-at "$STAMP"

python3 "$ROOT_DIR/scripts/verify_release_bundle.py" --bundle-dir "$BUNDLE_DIR"

printf '%s\n' "$BUNDLE_DIR" > "$ROOT_DIR/artifacts/sqlite/latest-bundle.txt"

echo "packaged bundle:"
echo "  $BUNDLE_DIR"
