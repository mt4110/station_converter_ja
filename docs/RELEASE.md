# RELEASE

## Goal

SQLite artifact を「export しただけ」で終わらせず、配布物としてまとめ、
GitHub Release asset まで載せるための導線です。

## Official path

1. primary write DB を最新化する
2. release に使う tag を決めて push する
3. `scripts/publish_sqlite_release.sh` で SQLite bundle を生成して GitHub Release に upload する

PostgreSQL を primary write にしている場合は次です。

```bash
./scripts/publish_sqlite_release.sh postgres v0.1.4
```

MySQL を primary write にしている場合は次です。

```bash
./scripts/publish_sqlite_release.sh mysql v0.1.4
```

この script は次をまとめて行います。

- `station-ops export-sqlite` で SQLite を再生成
- `scripts/package_sqlite_release.sh` で release-grade bundle を作成
- 既存 tag に紐づく GitHub Release を作成または再利用
- `stations.sqlite3`, `manifest.json`, `SOURCE_METADATA.json`, `checksums.txt`,
  `CHANGELOG.md`, `RELEASE_NOTES.md`, `README_SQLITE.md`, `SBOM.spdx.json` を upload

前提:

- `gh` CLI が入っていること
- `gh auth login` 済みであること
- 指定する tag が **現在の HEAD と同じ commit** を向いていること
- tag が remote に push 済みであること
- 公開済み tag を載せ替えず、新しい patch tag を使うこと

上の例は、最新 tag が `v0.1.3` の状態から `v0.1.4` を切る想定です。

## Local-only bundle

GitHub Release へはまだ上げず、ローカルで bundle だけ作りたい場合は従来どおり次です。

```bash
./scripts/release_sqlite_artifact.sh postgres
```

MySQL の場合:

```bash
./scripts/release_sqlite_artifact.sh mysql
```

## Tag discipline

publish script は、指定 tag が現在の `HEAD` を向いていない場合に失敗します。
公開済み tag が古い commit を向いている状態で release 内容を増やしたいときは、
公開 tag を載せ替えるよりも次の patch tag を切る方が事故が少なくなります。

`v0.1.0` は初回公開 tag としてそのまま残します。
release trust hardening を入れた成果物は、`v0.1.x` の新しい patch tag で公開してください。

## Outputs

`artifacts/sqlite/` に次を生成します。

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

manifest には少なくとも次を入れます。

- `source_url`
- `source_version`
- `source_sha256`
- `generated_at`
- row counts
- `git_commit`
- `tool_version`

詳しい bundle 中身は [`docs/ARTIFACTS.md`](./ARTIFACTS.md) を参照してください。

## Consumer verification

GitHub Release から asset を取得して検証する最短経路です。

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
shasum -a 256 -c checksums.txt
gh attestation verify stations.sqlite3 -R "$REPO"
gh attestation verify stations.sqlite3 -R "$REPO" \
  --predicate-type https://spdx.dev/Document/v2.3
```

Linux で `sha256sum` がある環境なら、checksum は次でも同じです。

```bash
sha256sum -c checksums.txt
```

`manifest.json` と `SOURCE_METADATA.json` で source snapshot、row counts、git commit を確認できます。
この artifact が主張する freshness は latest available MLIT N02 snapshot であり、
real-time railway data ではありません。

## GitHub Release workflow

`v*` tag push では `.github/workflows/release-sqlite.yml` が動きます。

- PostgreSQL service を立てる
- upstream N02 を ingest する
- SQLite artifact を export する
- release bundle を生成する
- `stations.sqlite3` に provenance attestation を付ける
- `SBOM.spdx.json` を使って SBOM attestation を付ける
- GitHub Release asset を publish / refresh する

pull request から release publish はしません。

## CI / local verification

主要経路の確認は次で揃います。

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
cd frontend && npm ci && npm run build
```

`verify_ingest_export.sh` は repo 内の小さな N02 fixture を ZIP 化して使うので、
外部 upstream に依存せず `migrate -> ingest -> export` を再現できます。
`verify_repo.sh` は Rust 側の verify に加えて、OpenAPI から再生成される
frontend の station SDK / 型定義の freshness も確認します。
