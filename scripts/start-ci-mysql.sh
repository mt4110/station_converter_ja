#!/usr/bin/env bash
set -euo pipefail

image=station-ci-mysql:local
container=station-ci-mysql
report=$(mktemp "${RUNNER_TEMP:?}/station-mysql-trivy.XXXXXX")
docker build --pull --tag "$image" deploy/docker/mysql-ci

# Oracle Linux RPMs require an advisory source that supports that distribution.
trivy image --config /dev/null --ignorefile /dev/null --image-src docker \
  --scanners vuln --list-all-pkgs --format json --output "$report" "$image"
jq -r '.Results[]? | [.Target, (.Packages | length), (.Vulnerabilities | length)] | @tsv' "$report"
jq -e '
  .Metadata.OS.Family == "oracle"
  and any(.Results[]; .Type == "oracle" and (.Packages | length) > 0)
  and all(.Results[]; (.Vulnerabilities | length) == 0)
' "$report" >/dev/null

docker run --detach --rm --name "$container" --publish 127.0.0.1:3214:3306 \
  --env MYSQL_ROOT_PASSWORD=root_password --env MYSQL_DATABASE=station_db \
  --env MYSQL_USER=station_user --env MYSQL_PASSWORD=station_password "$image"
for ((attempt = 0; attempt < 60; attempt++)); do
  if docker exec "$container" mysql --protocol=TCP -h 127.0.0.1 \
    -u station_user -pstation_password station_db -e 'SELECT 1' >/dev/null 2>&1; then
    exit 0
  fi
  sleep 2
done
docker logs "$container" >&2
exit 1
