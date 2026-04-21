#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import textwrap
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

SOURCE_NAME = "ksj_n02_station"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build a release-grade SQLite bundle with provenance metadata."
    )
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--sqlite-path", required=True)
    parser.add_argument("--bundle-dir", required=True)
    parser.add_argument("--release-version", required=True)
    parser.add_argument("--generated-at", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    repo_root = Path(args.repo_root).resolve()
    sqlite_path = Path(args.sqlite_path).resolve()
    bundle_dir = Path(args.bundle_dir).resolve()
    generated_at = args.generated_at
    release_version = args.release_version

    if not sqlite_path.is_file():
        raise SystemExit(f"SQLite artifact not found: {sqlite_path}")

    if bundle_dir.exists():
        shutil.rmtree(bundle_dir)
    bundle_dir.mkdir(parents=True, exist_ok=True)

    copied_sqlite = bundle_dir / "stations.sqlite3"
    shutil.copy2(sqlite_path, copied_sqlite)

    workspace_version = read_workspace_version(repo_root / "Cargo.toml")
    git_commit = git_output(repo_root, "rev-parse", "HEAD")
    git_short_commit = git_output(repo_root, "rev-parse", "--short", "HEAD")
    git_describe = git_output(repo_root, "describe", "--tags", "--always", "--dirty")
    git_remote_url = git_output_optional(repo_root, "remote", "get-url", "origin")
    git_repo_slug = parse_github_slug(git_remote_url) or os.environ.get("GITHUB_REPOSITORY")
    git_dirty = git_describe.endswith("-dirty")

    metrics = collect_sqlite_metrics(copied_sqlite)
    latest_snapshot = metrics["latest_snapshot"]
    if latest_snapshot is None:
        raise SystemExit("release bundle requires at least one source snapshot")

    build_provenance = collect_build_provenance(git_repo_slug, git_remote_url)
    included_files = [
        "stations.sqlite3",
        "manifest.json",
        "SOURCE_METADATA.json",
        "CHANGELOG.md",
        "RELEASE_NOTES.md",
        "README_SQLITE.md",
        "SBOM.spdx.json",
    ]

    manifest = {
        "schema_version": 1,
        "artifact_name": "station_converter_ja SQLite artifact",
        "artifact_kind": "sqlite_release_bundle",
        "release_version": release_version,
        "generated_at": generated_at,
        "git_commit": git_commit,
        "tool_version": workspace_version,
        "source_name": latest_snapshot["source_name"],
        "source_kind": latest_snapshot["source_kind"],
        "source_version": latest_snapshot["source_version"],
        "source_url": latest_snapshot["source_url"],
        "source_sha256": latest_snapshot["source_sha256"],
        "source_downloaded_at": latest_snapshot["downloaded_at"],
        "row_counts": metrics["row_counts"],
        "snapshot_counts": metrics["snapshot_counts"],
        "latest_snapshot_change_summary": metrics["latest_snapshot_change_summary"],
        "included_files": included_files + ["checksums.txt"],
        "provenance": {
            "repository": {
                "remote_url": git_remote_url,
                "github_repository": git_repo_slug,
                "commit": git_commit,
                "short_commit": git_short_commit,
                "describe": git_describe,
                "dirty": git_dirty,
            },
            "build": build_provenance,
        },
    }

    source_metadata = {
        "schema_version": 1,
        "canonical_source_name": SOURCE_NAME,
        "latest_source_snapshot": latest_snapshot,
        "source_snapshots": metrics["source_snapshots"],
        "latest_snapshot_change_summary": metrics["latest_snapshot_change_summary"],
    }

    changelog_src = repo_root / "CHANGELOG.md"
    if not changelog_src.is_file():
        raise SystemExit(f"CHANGELOG.md not found under {repo_root}")
    shutil.copy2(changelog_src, bundle_dir / "CHANGELOG.md")

    write_json(bundle_dir / "manifest.json", manifest)
    write_json(bundle_dir / "SOURCE_METADATA.json", source_metadata)
    write_text(
        bundle_dir / "README_SQLITE.md",
        render_readme_sqlite(release_version, generated_at, manifest, git_repo_slug),
    )
    write_text(
        bundle_dir / "RELEASE_NOTES.md",
        render_release_notes(release_version, generated_at, manifest, git_repo_slug),
    )

    sbom = build_spdx_sbom(
        bundle_dir=bundle_dir,
        release_version=release_version,
        generated_at=generated_at,
        git_commit=git_commit,
        git_repo_slug=git_repo_slug,
        described_files=[
            bundle_dir / "stations.sqlite3",
            bundle_dir / "manifest.json",
            bundle_dir / "SOURCE_METADATA.json",
            bundle_dir / "CHANGELOG.md",
            bundle_dir / "RELEASE_NOTES.md",
            bundle_dir / "README_SQLITE.md",
        ],
    )
    write_json(bundle_dir / "SBOM.spdx.json", sbom)

    write_checksums(
        bundle_dir / "checksums.txt",
        [
            bundle_dir / "stations.sqlite3",
            bundle_dir / "manifest.json",
            bundle_dir / "SOURCE_METADATA.json",
            bundle_dir / "CHANGELOG.md",
            bundle_dir / "RELEASE_NOTES.md",
            bundle_dir / "README_SQLITE.md",
            bundle_dir / "SBOM.spdx.json",
        ],
    )

    summary = {
        "bundle_dir": str(bundle_dir),
        "release_version": release_version,
        "generated_at": generated_at,
        "git_commit": git_commit,
        "source_version": latest_snapshot["source_version"],
        "source_url": latest_snapshot["source_url"],
        "active_station_count": metrics["row_counts"]["active_station_count"],
    }
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    return 0


def write_json(path: Path, payload: Dict[str, Any]) -> None:
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def write_text(path: Path, text: str) -> None:
    path.write_text(text.rstrip() + "\n", encoding="utf-8")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def write_checksums(output_path: Path, paths: List[Path]) -> None:
    lines = [f"{sha256_file(path)}  {path.name}" for path in paths]
    output_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def git_output(repo_root: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def git_output_optional(repo_root: Path, *args: str) -> Optional[str]:
    try:
        return git_output(repo_root, *args)
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None


def parse_github_slug(remote_url: Optional[str]) -> Optional[str]:
    if not remote_url:
        return None

    https_match = re.search(r"github\.com[:/](?P<slug>[^/]+/[^/.]+)(?:\.git)?$", remote_url)
    if https_match:
        return https_match.group("slug")

    ssh_match = re.search(r"git@github\.com:(?P<slug>[^/]+/[^/.]+)(?:\.git)?$", remote_url)
    if ssh_match:
        return ssh_match.group("slug")

    return None


def read_workspace_version(cargo_toml: Path) -> str:
    text = cargo_toml.read_text(encoding="utf-8")
    workspace_match = re.search(
        r"(?ms)^\[workspace\.package\]\s+.*?^version = \"([^\"]+)\"",
        text,
    )
    if not workspace_match:
        raise SystemExit(f"failed to parse workspace.package.version from {cargo_toml}")
    return workspace_match.group(1)


def collect_sqlite_metrics(sqlite_path: Path) -> Dict[str, Any]:
    connection = sqlite3.connect(sqlite_path)
    connection.row_factory = sqlite3.Row
    try:
        latest_snapshot = fetch_optional_row(
            connection,
            """
            SELECT id, source_name, source_kind, source_version, source_url, source_sha256, downloaded_at
            FROM source_snapshots
            WHERE source_name = ?
            ORDER BY id DESC
            LIMIT 1
            """,
            (SOURCE_NAME,),
        )
        row_counts = {
            "active_station_count": fetch_scalar(connection, "SELECT COUNT(*) FROM stations_latest"),
            "distinct_station_name_count": fetch_scalar(
                connection, "SELECT COUNT(DISTINCT station_name) FROM stations_latest"
            ),
            "distinct_line_count": fetch_scalar(
                connection, "SELECT COUNT(DISTINCT line_name) FROM stations_latest"
            ),
            "distinct_operator_count": fetch_scalar(
                connection, "SELECT COUNT(DISTINCT operator_name) FROM stations_latest"
            ),
            "station_identity_count": fetch_scalar(
                connection, "SELECT COUNT(*) FROM station_identities"
            ),
            "station_version_count": fetch_scalar(
                connection, "SELECT COUNT(*) FROM station_versions"
            ),
            "station_change_event_count": fetch_scalar(
                connection, "SELECT COUNT(*) FROM station_change_events"
            ),
        }
        snapshot_counts = {
            "source_snapshot_count": fetch_scalar(connection, "SELECT COUNT(*) FROM source_snapshots"),
            "active_version_snapshot_count": fetch_scalar(
                connection, "SELECT COUNT(DISTINCT snapshot_id) FROM stations_latest"
            ),
        }
        source_snapshots = fetch_rows(
            connection,
            """
            SELECT id, source_name, source_kind, source_version, source_url, source_sha256, downloaded_at
            FROM source_snapshots
            ORDER BY id
            """,
        )
        latest_snapshot_change_summary = {}
        if latest_snapshot is not None:
            latest_snapshot_change_summary = {
                row["change_kind"]: row["count"]
                for row in connection.execute(
                    """
                    SELECT change_kind, COUNT(*) AS count
                    FROM station_change_events
                    WHERE snapshot_id = ?
                    GROUP BY change_kind
                    ORDER BY change_kind
                    """,
                    (latest_snapshot["id"],),
                ).fetchall()
            }

        return {
            "latest_snapshot": latest_snapshot,
            "row_counts": row_counts,
            "snapshot_counts": snapshot_counts,
            "source_snapshots": source_snapshots,
            "latest_snapshot_change_summary": latest_snapshot_change_summary,
        }
    finally:
        connection.close()


def fetch_scalar(connection: sqlite3.Connection, query: str) -> int:
    row = connection.execute(query).fetchone()
    if row is None:
        raise SystemExit(f"query returned no rows: {query}")
    return int(row[0])


def fetch_optional_row(
    connection: sqlite3.Connection, query: str, params: Tuple[Any, ...]
) -> Optional[Dict[str, Any]]:
    row = connection.execute(query, params).fetchone()
    return row_to_dict(row) if row is not None else None


def fetch_rows(connection: sqlite3.Connection, query: str) -> List[Dict[str, Any]]:
    return [row_to_dict(row) for row in connection.execute(query).fetchall()]


def row_to_dict(row: sqlite3.Row) -> Dict[str, Any]:
    return {key: row[key] for key in row.keys()}


def collect_build_provenance(
    git_repo_slug: Optional[str], git_remote_url: Optional[str]
) -> Dict[str, Any]:
    github_repository = os.environ.get("GITHUB_REPOSITORY") or git_repo_slug
    github_server_url = os.environ.get("GITHUB_SERVER_URL")
    run_id = os.environ.get("GITHUB_RUN_ID")
    run_attempt = os.environ.get("GITHUB_RUN_ATTEMPT")
    workflow = os.environ.get("GITHUB_WORKFLOW")
    event_name = os.environ.get("GITHUB_EVENT_NAME")
    ref_name = os.environ.get("GITHUB_REF_NAME")

    if not any([run_id, workflow, event_name, ref_name]):
        return {
            "kind": "local",
            "repository": git_repo_slug,
            "remote_url": git_remote_url,
        }

    run_url = None
    if github_server_url and github_repository and run_id:
        run_url = f"{github_server_url}/{github_repository}/actions/runs/{run_id}"

    return {
        "kind": "github_actions",
        "repository": github_repository,
        "workflow": workflow,
        "event_name": event_name,
        "ref_name": ref_name,
        "run_id": run_id,
        "run_attempt": run_attempt,
        "run_url": run_url,
    }


def render_release_notes(
    release_version: str,
    generated_at: str,
    manifest: Dict[str, Any],
    git_repo_slug: Optional[str],
) -> str:
    row_counts = manifest["row_counts"]
    snapshot_counts = manifest["snapshot_counts"]
    repo_hint = git_repo_slug or "<owner/repo>"
    verification_commands = render_consumer_verification_commands(
        release_version=release_version,
        repo_hint=repo_hint,
    )
    verification_block = textwrap.indent(verification_commands, "        ")

    return textwrap.dedent(
        f"""
        # Release Notes

        ## Artifact

        - release_version: `{release_version}`
        - generated_at: `{generated_at}`
        - git_commit: `{manifest["git_commit"]}`
        - tool_version: `{manifest["tool_version"]}`

        ## Source Snapshot

        - source_name: `{manifest["source_name"]}`
        - source_version: `{manifest["source_version"] or "unknown"}`
        - source_url: `{manifest["source_url"]}`
        - source_sha256: `{manifest["source_sha256"]}`
        - downloaded_at: `{manifest["source_downloaded_at"]}`

        ## Dataset Summary

        - active_station_count: `{row_counts["active_station_count"]}`
        - distinct_station_name_count: `{row_counts["distinct_station_name_count"]}`
        - distinct_line_count: `{row_counts["distinct_line_count"]}`
        - distinct_operator_count: `{row_counts["distinct_operator_count"]}`
        - source_snapshot_count: `{snapshot_counts["source_snapshot_count"]}`
        - active_version_snapshot_count: `{snapshot_counts["active_version_snapshot_count"]}`
        - station_identity_count: `{row_counts["station_identity_count"]}`
        - station_version_count: `{row_counts["station_version_count"]}`
        - station_change_event_count: `{row_counts["station_change_event_count"]}`

        ## Included Assets

        - `stations.sqlite3`
        - `manifest.json`
        - `SOURCE_METADATA.json`
        - `checksums.txt`
        - `CHANGELOG.md`
        - `RELEASE_NOTES.md`
        - `README_SQLITE.md`
        - `SBOM.spdx.json`

        ## Verification

        ```bash
{verification_block}
        ```

        This artifact represents the latest available MLIT N02 snapshot included
        in this release. It is not real-time railway data.
        """
    ).strip()


def render_readme_sqlite(
    release_version: str,
    generated_at: str,
    manifest: Dict[str, Any],
    git_repo_slug: Optional[str],
) -> str:
    repo_hint = git_repo_slug or "<owner/repo>"
    source_version = manifest["source_version"] or "unknown"
    verification_commands = render_consumer_verification_commands(
        release_version=release_version,
        repo_hint=repo_hint,
    )
    verification_block = textwrap.indent(verification_commands, "        ")

    return textwrap.dedent(
        f"""
        # station_converter_ja SQLite Artifact

        この bundle は read-only SQLite artifact です。

        - release_version: `{release_version}`
        - generated_at: `{generated_at}`
        - source_version: `{source_version}`
        - git_commit: `{manifest["git_commit"]}`

        GitHub Release から取得して検証する場合:

        ```bash
{verification_block}
        ```

        SQLite を開く例:

        ```bash
        sqlite3 stations.sqlite3
        ```

        代表的な query:

        ```sql
        SELECT station_name, line_name, operator_name
        FROM stations_latest
        WHERE station_name LIKE '新宿%'
        ORDER BY operator_name, line_name, station_name
        LIMIT 20;
        ```

        provenance と source metadata は `manifest.json` と `SOURCE_METADATA.json` を参照してください。
        この artifact は latest available MLIT N02 snapshot の配布物であり、real-time railway data ではありません。
        """
    ).strip()


def render_consumer_verification_commands(*, release_version: str, repo_hint: str) -> str:
    return textwrap.dedent(
        f"""
        REPO="{repo_hint}"
        TAG="{release_version}"
        mkdir -p "tmp/release-${{TAG}}"
        gh release download "$TAG" -R "$REPO" -D "tmp/release-${{TAG}}" --clobber \\
          -p stations.sqlite3 \\
          -p manifest.json \\
          -p SOURCE_METADATA.json \\
          -p checksums.txt \\
          -p CHANGELOG.md \\
          -p RELEASE_NOTES.md \\
          -p README_SQLITE.md \\
          -p SBOM.spdx.json
        cd "tmp/release-${{TAG}}"
        shasum -a 256 -c checksums.txt
        gh attestation verify stations.sqlite3 -R "$REPO"
        gh attestation verify stations.sqlite3 -R "$REPO" \\
          --predicate-type https://spdx.dev/Document/v2.3
        """
    ).strip()


def build_spdx_sbom(
    *,
    bundle_dir: Path,
    release_version: str,
    generated_at: str,
    git_commit: str,
    git_repo_slug: Optional[str],
    described_files: List[Path],
) -> Dict[str, Any]:
    namespace_owner = git_repo_slug or "local/station_converter_ja"
    package_spdx_id = "SPDXRef-Package-release-bundle"

    files = []
    relationships = [
        {
            "spdxElementId": "SPDXRef-DOCUMENT",
            "relationshipType": "DESCRIBES",
            "relatedSpdxElement": package_spdx_id,
        }
    ]

    for index, path in enumerate(described_files, start=1):
        spdx_id = f"SPDXRef-File-{index}"
        files.append(
            {
                "fileName": f"./{path.relative_to(bundle_dir).as_posix()}",
                "SPDXID": spdx_id,
                "checksums": [
                    {
                        "algorithm": "SHA256",
                        "checksumValue": sha256_file(path),
                    }
                ],
                "licenseConcluded": "NOASSERTION",
                "licenseInfoInFiles": ["NOASSERTION"],
                "copyrightText": "NOASSERTION",
            }
        )
        relationships.append(
            {
                "spdxElementId": package_spdx_id,
                "relationshipType": "CONTAINS",
                "relatedSpdxElement": spdx_id,
            }
        )

    return {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": f"station_converter_ja SQLite release bundle {release_version}",
        "documentNamespace": (
            f"https://github.com/{namespace_owner}/releases/{release_version}/sbom/{git_commit}"
        ),
        "creationInfo": {
            "created": generated_at,
            "creators": ["Tool: station_converter_ja release bundler"],
        },
        "documentDescribes": [package_spdx_id],
        "packages": [
            {
                "name": f"station_converter_ja SQLite release bundle {release_version}",
                "SPDXID": package_spdx_id,
                "downloadLocation": "NOASSERTION",
                "filesAnalyzed": True,
                "licenseConcluded": "NOASSERTION",
                "licenseDeclared": "NOASSERTION",
                "copyrightText": "NOASSERTION",
                "versionInfo": release_version,
            }
        ],
        "files": files,
        "relationships": relationships,
    }


if __name__ == "__main__":
    sys.exit(main())
