# CONTRIBUTING

この repo で変更を入れる前に、まず [AGENTS.md](AGENTS.md) を読んでください。
データ方針、DB の責務、release の扱い、public API の更新ルールをそこに固定しています。

## Local workflow

1. DB を選ぶ
   - `postgres` または `mysql`
   - `sqlite` はローカル確認用
2. env と DB を用意する

```bash
./scripts/setup_nix_docker.sh postgres
nix develop
```

3. migrate -> ingest -> export を通す

```bash
cargo run -p station-ops -- migrate
cargo run -p station-ops -- job ingest-n02 --export-sqlite
```

4. 変更に応じて verify を回す

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
cargo run -p station-ops -- validate-ingest --strict --json
cd frontend && npm ci && npm run build
```

`./scripts/verify_repo.sh` は Rust 側の verify に加えて
`frontend` の `npm run verify:station-sdk` も実行し、
OpenAPI 変更時に generated SDK / 型定義の取りこぼしを検出します。

## What to update with your change

- runtime / operational behavior を変えたら `README.md` と該当 docs を更新する
- public API を変えたら `API_SPEC.md` を更新する
- OpenAPI 導入後は generated OpenAPI も必ず同期する
- OpenAPI / API contract の設計方針は `docs/OPENAPI.md` を参照する
- release artifact の中身を変えたら `docs/RELEASE.md` と `docs/ROADMAP.md` を見直す
- source / license policy を変えたら `docs/SOURCE_POLICY.md` を更新する

## Before you open a PR

- verify scripts が通ること
- docs の導線が壊れていないこと
- `N02` canonical / `N05` optional non-commercial overlay の線引きを崩していないこと
- PostgreSQL / MySQL が primary write、SQLite が read-only artifact という前提を崩していないこと
