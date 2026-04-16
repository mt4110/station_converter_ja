#!/usr/bin/env bash
set -euo pipefail

cp -n worker/api/.env.example worker/api/.env || true
cp -n worker/crawler/.env.example worker/crawler/.env || true
cp -n worker/ops/.env.example worker/ops/.env || true
cp -n frontend/.env.local.example frontend/.env.local || true

echo "onboard complete"
echo "next:"
echo "  1) docker compose up -d"
echo "  2) nix develop"
echo "  3) cargo run -p station-ops -- migrate"
