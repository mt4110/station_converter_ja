#!/usr/bin/env bash
set -euo pipefail

./scripts/onboard.sh
docker compose up -d
echo "docker services started"
echo "enter dev shell with: nix develop"
