# ARTIFACTS

## SQLite release bundle

`./scripts/release_sqlite_artifact.sh postgres` または
`./scripts/publish_sqlite_release.sh postgres <tag>` で、次の bundle を生成します。

```text
artifacts/sqlite/station_converter_ja-<release-version>-sqlite-<timestamp>/
  stations.sqlite3
  manifest.json
  SOURCE_METADATA.json
  checksums.txt
  CHANGELOG.md
  RELEASE_NOTES.md
  README_SQLITE.md
  SBOM.spdx.json
```

## What each file is for

- `stations.sqlite3`
  - 配布用の read-only SQLite artifact
- `manifest.json`
  - source snapshot、row counts、git commit、tool version、build provenance を記録
- `SOURCE_METADATA.json`
  - export に入った `source_snapshots` と最新 snapshot の詳細
- `checksums.txt`
  - release asset の SHA-256
- `CHANGELOG.md`
  - repo の change history
- `RELEASE_NOTES.md`
  - その release の source version と dataset summary
- `README_SQLITE.md`
  - SQLite artifact の最短利用手順
- `SBOM.spdx.json`
  - SPDX 2.3 形式の SBOM

## Verification

SHA-256:

```bash
sha256sum -c checksums.txt
```

artifact attestation:

```bash
gh attestation verify stations.sqlite3 -R mt4110/station_converter_ja
```

SBOM attestation:

```bash
gh attestation verify stations.sqlite3 \
  -R mt4110/station_converter_ja \
  --predicate-type https://spdx.dev/Document/v2.3
```

## Release workflow

tag push (`v*`) では `.github/workflows/release-sqlite.yml` が走り、
PostgreSQL 上で ingest -> SQLite export -> bundle packaging -> GitHub Release upload を行います。

この workflow は pull request から release publish しません。
