# Release Checklist

This checklist is the last-mile runbook before publishing a SQLite release.
Use a new patch tag. Do not move published tags.

## Release Paths

Choose one path before starting.

- **Local candidate only**
  - Build and verify the bundle under `artifacts/sqlite/`
  - Do not create or push a tag
  - Do not upload GitHub Release assets
- **Tag push workflow**
  - Preferred path for normal public releases
  - Push a `v*` tag from the release commit
  - `.github/workflows/release-sqlite.yml` builds, attests, and publishes assets
- **Manual publish**
  - Repair path when the workflow needs to be reproduced from a clean tagged checkout
  - Requires `gh` auth and a tag that points at the current `HEAD`
  - Uses `./scripts/publish_sqlite_release.sh <db> <tag>`

## Stop Conditions

Stop before publishing if any item is true.

- The working tree is dirty or has untracked release changes
- The release tag does not point at the current `HEAD`
- The tag has already been published and the desired content changed
- `manifest.json` / `SOURCE_METADATA.json` points at the wrong source snapshot
- Validation or SQLite parity fails
- The release notes imply real-time railway data freshness
- `N05` data has been silently mixed into canonical export

If a public tag already exists and the release content needs to change, cut a new
patch tag instead of moving the old tag.

## Variables

Set these once for the release shell.

```bash
REPO=mt4110/station_converter_ja
DB_TYPE=postgres
TAG=v0.1.x
```

Use `DB_TYPE=mysql` only when the release primary write DB is MySQL.

## 1. Preflight

Confirm the release candidate starts from a clean, current branch.

```bash
git status --short --branch
git fetch origin --tags
git log --oneline --decorate -5
```

Check GitHub CLI auth before the final publish path.

```bash
gh auth status
```

## 2. Repository Verification

Run the standard local gates.

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
cd frontend && npm ci && npm run build
```

## 3. Release Database Verification

On the release database, refresh the source snapshot and verify the export.

```bash
cargo run -p station-ops -- job refresh-n02 --export-sqlite
cargo run -p station-ops -- validate-ingest --strict --json
cargo run -p station-ops -- verify-sqlite-parity
./scripts/verify_sqlite_reproducibility.sh
```

Confirm the candidate freshness claim is only latest available MLIT N02 snapshot,
not real-time railway data.

## 4. Local Candidate Bundle

Build a release-grade bundle without publishing.

```bash
./scripts/release_sqlite_artifact.sh "$DB_TYPE" "$TAG"
```

Inspect the generated bundle.

```bash
BUNDLE_DIR="$(cat artifacts/sqlite/latest-bundle.txt)"
python3 scripts/verify_release_bundle.py --bundle-dir "$BUNDLE_DIR"
sed -n '1,120p' "$BUNDLE_DIR/RELEASE_NOTES.md"
sed -n '1,120p' "$BUNDLE_DIR/SOURCE_METADATA.json"
```

The bundle must include:

- `stations.sqlite3`
- `manifest.json`
- `SOURCE_METADATA.json`
- `checksums.txt`
- `CHANGELOG.md`
- `RELEASE_NOTES.md`
- `README_SQLITE.md`
- `SBOM.spdx.json`

## 5. Tag

Create the release tag only after the candidate is accepted.

```bash
git tag -a "$TAG" -m "$TAG"
git rev-parse HEAD
git rev-parse "${TAG}^{commit}"
```

The two commits must match.

## 6. Publish

For the normal workflow path, push the tag. This triggers GitHub Release publish.

```bash
git push origin "$TAG"
```

For the manual repair path, run the publish script from the clean tagged commit.
This uploads or refreshes GitHub Release assets.

```bash
./scripts/publish_sqlite_release.sh "$DB_TYPE" "$TAG"
```

## 7. Consumer Verification

After the release is public, verify the downloadable assets as a consumer.

```bash
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
shasum -a 256 -c checksums.txt
gh attestation verify stations.sqlite3 -R "$REPO"
gh attestation verify stations.sqlite3 -R "$REPO" \
  --predicate-type https://spdx.dev/Document/v2.3
```

On Linux, `sha256sum -c checksums.txt` is equivalent to the `shasum` command.
