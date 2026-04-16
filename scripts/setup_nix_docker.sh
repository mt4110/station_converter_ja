#!/usr/bin/env bash
set -euo pipefail

DB_TYPE="${1:-postgres}"

./scripts/onboard.sh "$DB_TYPE"
docker compose up -d "$DB_TYPE"
echo "docker services started"
echo "enter dev shell with: nix develop"
