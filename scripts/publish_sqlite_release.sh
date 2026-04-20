#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 [postgres|mysql] <release-tag>" >&2
  echo "example: $0 postgres v0.1.1" >&2
}

DB_TYPE="${1:-}"
RELEASE_TAG="${2:-}"

case "$DB_TYPE" in
  postgres|mysql)
    ;;
  *)
    usage
    exit 1
    ;;
esac

if [[ -z "$RELEASE_TAG" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI not found; install GitHub CLI first" >&2
  exit 1
fi

if ! git rev-parse -q --verify "${RELEASE_TAG}^{commit}" >/dev/null 2>&1; then
  echo "git tag not found locally: ${RELEASE_TAG}" >&2
  echo "create or fetch the tag before publishing assets" >&2
  exit 1
fi

HEAD_COMMIT="$(git rev-parse HEAD)"
TAG_COMMIT="$(git rev-parse "${RELEASE_TAG}^{commit}")"

if [[ "$HEAD_COMMIT" != "$TAG_COMMIT" ]]; then
  echo "release tag ${RELEASE_TAG} points to ${TAG_COMMIT}, but HEAD is ${HEAD_COMMIT}" >&2
  echo "checkout the tagged commit or cut a new tag before publishing assets" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain --untracked-files=normal --ignore-submodules=all)" ]]; then
  echo "working tree must be clean before publishing release assets" >&2
  echo "commit, stash, or remove staged, unstaged, and untracked changes, then retry" >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "gh is not authenticated; run: gh auth login" >&2
  exit 1
fi

"$ROOT_DIR/scripts/release_sqlite_artifact.sh" "$DB_TYPE" "$RELEASE_TAG"

LATEST_BUNDLE_FILE="$ROOT_DIR/artifacts/sqlite/latest-bundle.txt"
if [[ ! -f "$LATEST_BUNDLE_FILE" ]]; then
  echo "latest bundle marker not found: $LATEST_BUNDLE_FILE" >&2
  exit 1
fi

BUNDLE_DIR="$(cat "$LATEST_BUNDLE_FILE")"
if [[ ! -d "$BUNDLE_DIR" ]]; then
  echo "bundle directory not found: $BUNDLE_DIR" >&2
  exit 1
fi

ASSET_PATHS=(
  "$BUNDLE_DIR/stations.sqlite3"
  "$BUNDLE_DIR/manifest.json"
  "$BUNDLE_DIR/SOURCE_METADATA.json"
  "$BUNDLE_DIR/checksums.txt"
  "$BUNDLE_DIR/CHANGELOG.md"
  "$BUNDLE_DIR/RELEASE_NOTES.md"
  "$BUNDLE_DIR/README_SQLITE.md"
  "$BUNDLE_DIR/SBOM.spdx.json"
)

for path in "${ASSET_PATHS[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "missing release bundle file: $path" >&2
    exit 1
  fi
done

if ! gh release view "$RELEASE_TAG" >/dev/null 2>&1; then
  gh release create "$RELEASE_TAG" \
    "${ASSET_PATHS[@]}" \
    --verify-tag \
    --title "$RELEASE_TAG" \
    --notes-file "$BUNDLE_DIR/RELEASE_NOTES.md"
else
  gh release upload "$RELEASE_TAG" "${ASSET_PATHS[@]}" --clobber
  gh release edit "$RELEASE_TAG" \
    --title "$RELEASE_TAG" \
    --notes-file "$BUNDLE_DIR/RELEASE_NOTES.md"
fi

echo "uploaded release assets for ${RELEASE_TAG}:"
for path in "${ASSET_PATHS[@]}"; do
  echo "  $path"
done
