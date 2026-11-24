#!/usr/bin/env bash
set -euo pipefail

# OSV-Scanner v2.7.1, linux/amd64. Verify the release asset before executing it.
version=v2.7.1
checksum=fb7dab19a8bd9bbe5c3189f09d4b7ee1820c9553a23bac202648a56efd2f6ee8
install_dir=$(mktemp -d "${RUNNER_TEMP:?}/osv.XXXXXX")
binary="$install_dir/osv-scanner"
curl --fail --silent --show-error --location --retry 3 \
  "https://github.com/google/osv-scanner/releases/download/$version/osv-scanner_linux_amd64" \
  --output "$binary"
printf '%s  %s\n' "$checksum" "$binary" | sha256sum --check -
chmod +x "$binary"
printf '%s\n' "$install_dir" >> "${GITHUB_PATH:?}"
