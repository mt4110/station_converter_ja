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

GitHub Release から asset を取得します。

```bash
REPO=mt4110/station_converter_ja
TAG=v0.1.4
mkdir -p "tmp/release-${TAG}"
gh release download "$TAG" -R "$REPO" -D "tmp/release-${TAG}" --clobber \
  -p stations.sqlite3 \
  -p manifest.json \
  -p SOURCE_METADATA.json \
  -p checksums.txt \
  -p CHANGELOG.md \
  -p RELEASE_NOTES.md \
  -p README_SQLITE.md \
  -p SBOM.spdx.json
cd "tmp/release-${TAG}"
```

SHA-256:

```bash
shasum -a 256 -c checksums.txt
```

Linux で `sha256sum` がある場合:

```bash
sha256sum -c checksums.txt
```

artifact attestation:

```bash
gh attestation verify stations.sqlite3 -R "$REPO"
```

SBOM attestation:

```bash
gh attestation verify stations.sqlite3 \
  -R "$REPO" \
  --predicate-type https://spdx.dev/Document/v2.3
```

`manifest.json` は source snapshot、row counts、git commit、tool version を持ちます。
`SOURCE_METADATA.json` は export に入った source snapshot history を持ちます。
この bundle は latest available MLIT N02 snapshot を配るためのもので、
real-time railway data ではありません。

## Release workflow

tag push (`v*`) では `.github/workflows/release-sqlite.yml` が走り、
PostgreSQL 上で ingest -> SQLite export -> bundle packaging -> GitHub Release upload を行います。

この workflow は pull request から release publish しません。
