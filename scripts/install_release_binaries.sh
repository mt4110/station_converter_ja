#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" = "-h" || "${1:-}" = "--help" ]]; then
  echo "usage: $0 [install-root] [owner] [group]"
  echo "example: sudo $0 /opt/station_converter_ja station-converter-ja station-converter-ja"
  exit 0
fi

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
INSTALL_ROOT="${1:-/opt/station_converter_ja}"
OWNER="${2:-$(id -un)}"
GROUP="${3:-$(id -gn)}"

cd "$ROOT_DIR"

echo "building release binaries"
cargo build --release -p station-api -p station-ops

echo "installing binaries under ${INSTALL_ROOT}"
install -d -o "$OWNER" -g "$GROUP" \
  "$INSTALL_ROOT" \
  "$INSTALL_ROOT/target" \
  "$INSTALL_ROOT/target/release" \
  "$INSTALL_ROOT/storage/sqlite" \
  "$INSTALL_ROOT/worker/crawler/temp_assets"

install -m 0755 -o "$OWNER" -g "$GROUP" \
  target/release/station-api \
  "$INSTALL_ROOT/target/release/station-api"
install -m 0755 -o "$OWNER" -g "$GROUP" \
  target/release/station-ops \
  "$INSTALL_ROOT/target/release/station-ops"

echo "installed:"
echo "  $INSTALL_ROOT/target/release/station-api"
echo "  $INSTALL_ROOT/target/release/station-ops"
