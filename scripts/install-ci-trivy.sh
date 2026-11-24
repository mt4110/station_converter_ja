#!/usr/bin/env bash
set -euo pipefail

version=0.76.2
checksum=bc0fc726e017b8927a62b2d3531d7a84ee01769daf5c99ed866dced54089fdbf
install_dir=$(mktemp -d "${RUNNER_TEMP:?}/trivy.XXXXXX")
archive="$install_dir/trivy.tar.gz"
curl --fail --silent --show-error --location --retry 3 \
  "https://github.com/aquasecurity/trivy/releases/download/v$version/trivy_${version}_Linux-64bit.tar.gz" \
  --output "$archive"
printf '%s  %s\n' "$checksum" "$archive" | sha256sum --check -
tar -xzf "$archive" -C "$install_dir" trivy
printf '%s\n' "$install_dir" >> "${GITHUB_PATH:?}"
