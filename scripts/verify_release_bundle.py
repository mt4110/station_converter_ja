#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REQUIRED_FILES = [
    "stations.sqlite3",
    "manifest.json",
    "SOURCE_METADATA.json",
    "checksums.txt",
    "CHANGELOG.md",
    "RELEASE_NOTES.md",
    "README_SQLITE.md",
    "SBOM.spdx.json",
]

REQUIRED_MANIFEST_KEYS = [
    "generated_at",
    "git_commit",
    "tool_version",
    "source_url",
    "source_version",
    "source_sha256",
    "row_counts",
]

REQUIRED_CONSUMER_VERIFICATION_TEXT = [
    "gh release download",
    "shasum -a 256 -c checksums.txt",
    "gh attestation verify stations.sqlite3",
    "--predicate-type https://spdx.dev/Document/v2.3",
    "latest available MLIT N02 snapshot",
    "real-time railway data",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Verify a generated SQLite release bundle.")
    parser.add_argument("--bundle-dir", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    bundle_dir = Path(args.bundle_dir).resolve()

    if not bundle_dir.is_dir():
        raise SystemExit(f"bundle directory not found: {bundle_dir}")

    for file_name in REQUIRED_FILES:
        path = bundle_dir / file_name
        if not path.is_file():
            raise SystemExit(f"required bundle file missing: {path}")

    manifest = load_json(bundle_dir / "manifest.json")
    source_metadata = load_json(bundle_dir / "SOURCE_METADATA.json")
    sbom = load_json(bundle_dir / "SBOM.spdx.json")

    for key in REQUIRED_MANIFEST_KEYS:
        if key not in manifest:
            raise SystemExit(f"manifest.json missing key: {key}")

    row_counts = manifest["row_counts"]
    if not isinstance(row_counts, dict) or "active_station_count" not in row_counts:
        raise SystemExit("manifest.json row_counts is missing active_station_count")

    if "latest_source_snapshot" not in source_metadata:
        raise SystemExit("SOURCE_METADATA.json missing latest_source_snapshot")

    if sbom.get("spdxVersion") != "SPDX-2.3":
        raise SystemExit("SBOM.spdx.json must declare SPDX-2.3")

    verify_checksums(bundle_dir / "checksums.txt", bundle_dir)
    verify_consumer_docs(bundle_dir)

    summary = {
        "bundle_dir": str(bundle_dir),
        "active_station_count": row_counts["active_station_count"],
        "source_version": manifest["source_version"],
        "git_commit": manifest["git_commit"],
    }
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    return 0


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def verify_checksums(checksums_path: Path, bundle_dir: Path) -> None:
    lines = checksums_path.read_text(encoding="utf-8").splitlines()
    if not lines:
        raise SystemExit(f"checksums file is empty: {checksums_path}")

    for line in lines:
        parts = line.split("  ", 1)
        if len(parts) != 2:
            raise SystemExit(f"invalid checksum line: {line}")
        expected_hash, file_name = parts
        target = bundle_dir / file_name
        if not target.is_file():
            raise SystemExit(f"checksums target missing: {target}")
        actual_hash = sha256_file(target)
        if actual_hash != expected_hash:
            raise SystemExit(
                f"checksum mismatch for {target.name}: expected {expected_hash}, got {actual_hash}"
            )


def verify_consumer_docs(bundle_dir: Path) -> None:
    for file_name in ["RELEASE_NOTES.md", "README_SQLITE.md"]:
        text = (bundle_dir / file_name).read_text(encoding="utf-8")
        for required_text in REQUIRED_CONSUMER_VERIFICATION_TEXT:
            if required_text not in text:
                raise SystemExit(f"{file_name} missing consumer verification text: {required_text}")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


if __name__ == "__main__":
    sys.exit(main())
