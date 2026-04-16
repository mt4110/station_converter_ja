#!/usr/bin/env bash
set -euo pipefail

DB_TYPE="${1:-postgres}"

case "$DB_TYPE" in
  postgres|mysql|sqlite)
    ;;
  *)
    echo "usage: $0 [postgres|mysql|sqlite]" >&2
    exit 1
    ;;
esac

cp -n worker/api/.env.example worker/api/.env || true
cp -n worker/crawler/.env.example worker/crawler/.env || true
cp -n worker/ops/.env.example worker/ops/.env || true
cp -n frontend/.env.local.example frontend/.env.local || true
mkdir -p storage/locks storage/sqlite worker/crawler/temp_assets

for env_file in worker/api/.env worker/crawler/.env worker/ops/.env; do
  tmp_file="$(mktemp)"
  awk -v db_type="$DB_TYPE" '
    /^DATABASE_TYPE=/ { print "DATABASE_TYPE=" db_type; next }
    { print }
  ' "$env_file" > "$tmp_file"
  mv "$tmp_file" "$env_file"
done

if [ "$DB_TYPE" = "sqlite" ]; then
  docker_step="skip (sqlite uses no docker DB service)"
  ingest_step="cargo run -p station-ops -- job ingest-n02"
else
  docker_step="docker compose up -d ${DB_TYPE}"
  ingest_step="cargo run -p station-ops -- job ingest-n02 --export-sqlite"
fi

echo "onboard complete"
echo "next:"
echo "  1) ${docker_step}"
echo "  2) nix develop"
echo "  3) cargo run -p station-ops -- migrate"
echo "  4) ${ingest_step}"
echo "  5) cargo run -p station-api"
echo
echo "Rust services auto-load worker/*/.env, so the copied env files work as-is."
