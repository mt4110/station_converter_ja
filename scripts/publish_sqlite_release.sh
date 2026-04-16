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

if ! git diff --quiet --ignore-submodules -- || ! git diff --cached --quiet --ignore-submodules --; then
  echo "working tree must be clean before publishing release assets" >&2
  echo "commit or stash staged and unstaged changes, then retry" >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "gh is not authenticated; run: gh auth login" >&2
  exit 1
fi

"$ROOT_DIR/scripts/release_sqlite_artifact.sh" "$DB_TYPE"

LATEST_MANIFEST="$(find "$ROOT_DIR/artifacts/sqlite" -maxdepth 1 -type f -name 'manifest-*.txt' | sort | tail -n 1)"

if [[ -z "$LATEST_MANIFEST" ]]; then
  echo "no manifest found under artifacts/sqlite" >&2
  exit 1
fi

STAMP="${LATEST_MANIFEST##*/manifest-}"
STAMP="${STAMP%.txt}"
SQLITE_BUNDLE="$ROOT_DIR/artifacts/sqlite/stations-${STAMP}.sqlite3"
CHECKSUM_BUNDLE="$ROOT_DIR/artifacts/sqlite/checksums-${STAMP}.txt"

for path in "$SQLITE_BUNDLE" "$CHECKSUM_BUNDLE" "$LATEST_MANIFEST"; do
  if [[ ! -f "$path" ]]; then
    echo "missing release bundle file: $path" >&2
    exit 1
  fi
done

if ! gh release view "$RELEASE_TAG" >/dev/null 2>&1; then
  gh release create "$RELEASE_TAG" --verify-tag --generate-notes
fi

gh release upload "$RELEASE_TAG" \
  "$SQLITE_BUNDLE" \
  "$CHECKSUM_BUNDLE" \
  "$LATEST_MANIFEST" \
  --clobber

echo "uploaded release assets for ${RELEASE_TAG}:"
echo "  $SQLITE_BUNDLE"
echo "  $CHECKSUM_BUNDLE"
echo "  $LATEST_MANIFEST"
