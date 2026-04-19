#!/usr/bin/env bash
set -euo pipefail

DB_TYPE="${1:-postgres}"

./scripts/onboard.sh "$DB_TYPE"

if [ "$DB_TYPE" = "sqlite" ]; then
  echo "sqlite selected; no docker DB service to start"
else
  docker compose up -d "$DB_TYPE"
  echo "docker services started"
fi

echo "enter dev shell with: nix develop"
