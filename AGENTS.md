# AGENTS

このファイルは、repo 内で作業する contributor と local automation 向けの共通ルールです。
実装前に「この repo が何を守るのか」をここで揃えます。

## Product contract

- canonical source は **MLIT / 国土数値情報 `N02`**
- `N05` は **optional non-commercial overlay only**
- primary write DB は **PostgreSQL / MySQL**
- **SQLite は read-only artifact**
- **Redis は cache only**
- public な freshness claim は **latest available MLIT N02 snapshot**
  - real-time railway data とは言わない
  - 「今日の開業駅まで即反映」のような表現はしない

## Repo layout

- `worker/api/`
  - axum API
- `worker/ops/`
  - migrate / ingest / validate / export の CLI
- `worker/crawler/`
  - N02 fetch / parse の crawler と dev helper loop
- `worker/shared/`
  - config / DB / model shared code
- `frontend/`
  - example frontend
- `docs/`
  - 運用、DB、release、deploy、roadmap
- `deploy/`, `infra/`
  - self-hosted と将来向け skeleton
- `scripts/`
  - verify / release / install 補助
- `testdata/n02/`
  - ingest/export 検証 fixture

## Standard commands

初回セットアップ:

```bash
./scripts/setup_nix_docker.sh postgres
nix develop
```

主要フロー:

```bash
cargo run -p station-ops -- migrate
cargo run -p station-ops -- job ingest-n02 --export-sqlite
cargo run -p station-api
```

検証:

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
cargo run -p station-ops -- validate-ingest --strict --json
cd frontend && npm ci && npm run build
```

release:

```bash
./scripts/release_sqlite_artifact.sh postgres
./scripts/publish_sqlite_release.sh postgres <tag>
```

## Change rules

- public API を変えるなら `API_SPEC.md` を更新する
- generated OpenAPI 導入後は、public API change を **OpenAPI と `API_SPEC.md` の両方**に反映する
- generated artifact には最低でも **SQLite 本体、manifest、checksum** を含める
- release trust hardening では `SOURCE_METADATA.json`、SBOM、artifact attestation を追加する
- `N05` を canonical export に silently 混ぜない
- SQLite を primary write DB 扱いしない
- Redis に source of truth を置かない
- cloud production resources は **explicit review なしに本実装しない**
- release tag を載せ替えない
  - 既存 tag と release 内容が噛み合わなくなったら patch tag を切る

## Current documentation path

- contributor flow: [CONTRIBUTING.md](CONTRIBUTING.md)
- current priorities: [docs/ROADMAP.md](docs/ROADMAP.md)
- source policy: [docs/SOURCE_POLICY.md](docs/SOURCE_POLICY.md)
- operations: [docs/OPERATIONS.md](docs/OPERATIONS.md)

