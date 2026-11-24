#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# Include services from every Compose profile, and external Dockerfile stages.
compose_config=$(docker compose --profile '*' config --format json)
compose_images=$(jq -r '.services[] | select(.build == null) | .image' <<< "$compose_config")
compose_builds=$(jq -r '.services | to_entries[] | select(.value.build != null) | .key' <<< "$compose_config")
base_images=$(awk '
  FNR == 1 { for (stage in stages) delete stages[stage] }
  toupper($1) == "FROM" {
    image = ($2 ~ /^--platform=/) ? $3 : $2
    if (tolower(image) != "scratch" && !(tolower(image) in stages)) print image
    for (i = 3; i < NF; i++)
      if (toupper($i) == "AS") stages[tolower($(i + 1))] = 1
  }
' worker/*/Dockerfile deploy/docker/*/Dockerfile)
ci_images=$(awk '$1 == "image:" { print $2 }' .github/workflows/ci.yml .github/workflows/release-sqlite.yml)
[[ -n "$compose_images" && -n "$base_images" && -n "$ci_images" ]]
images=$(printf '%s\n' "$compose_images" "$base_images" "$ci_images" | sort -u)

status=0
while IFS= read -r image; do
  printf '\nScanning Docker input %s\n' "$image"
  if ! docker pull "$image" ||
     ! osv-scanner scan image --config /dev/null --all-vulns --format vertical -- "$image"; then
    status=1
  fi
done <<< "$images"

while IFS= read -r service; do
  [[ -n "$service" ]] || continue
  printf '\nBuilding and scanning Compose service %s\n' "$service"
  if ! docker compose --profile '*' build --pull "$service" ||
     ! image=$(docker compose --profile '*' config --images "$service") ||
     ! osv-scanner scan image --config /dev/null --all-vulns --format vertical -- "$image"; then
    status=1
  fi
done <<< "$compose_builds"
exit "$status"
