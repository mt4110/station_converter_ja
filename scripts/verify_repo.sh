#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
FRONTEND_DIR="$ROOT_DIR/frontend"
OPENAPI_TYPESCRIPT_BIN="$FRONTEND_DIR/node_modules/.bin/openapi-typescript"

ensure_frontend_sdk_tooling() {
  if [ -x "$OPENAPI_TYPESCRIPT_BIN" ]; then
    return
  fi

  if ! command -v npm >/dev/null 2>&1; then
    echo "npm is required to verify generated station SDK artifacts." >&2
    exit 1
  fi

  echo "installing frontend dependencies for station SDK verification"
  (
    cd "$FRONTEND_DIR"
    npm ci
  )
}

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
ensure_frontend_sdk_tooling

(
  cd "$FRONTEND_DIR"
  npm run verify:station-sdk
)
