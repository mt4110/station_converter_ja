#!/usr/bin/env bash
set -euo pipefail

root=${1:-$(git rev-parse --show-toplevel)}
status=0
for lockfile in Cargo.lock frontend/package-lock.json; do
  path="$root/$lockfile"
  if [[ ! -s "$path" ]]; then
    printf 'Missing or empty dependency lockfile: %s\n' "$lockfile" >&2
    status=1
    continue
  fi
  # Scan each lockfile separately so a successful extraction cannot mask a missing one.
  if ! osv-scanner scan source --lockfile "$path" --config /dev/null --all-vulns --format vertical; then
    status=1
  fi
done
exit "$status"
