#!/usr/bin/env bash
set -euo pipefail

image=station-ci-postgres:local
container=station-ci-postgres
docker build --pull --tag "$image" deploy/docker/postgres-ci
osv-scanner scan image --config /dev/null --all-vulns --format vertical "$image"

docker run --detach --rm --name "$container" --publish 127.0.0.1:3215:5432 \
  --env POSTGRES_DB=station_db --env POSTGRES_USER=postgres \
  --env POSTGRES_PASSWORD=postgres_password "$image"
for ((attempt = 0; attempt < 60; attempt++)); do
  if docker exec "$container" pg_isready -h 127.0.0.1 -U postgres -d station_db; then
    exit 0
  fi
  sleep 2
done
docker logs "$container" >&2
exit 1
