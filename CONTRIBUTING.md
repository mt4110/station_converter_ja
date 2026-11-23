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
- dependency security checks が通ること

```bash
./scripts/check-osv-lockfiles.sh
cargo deny check
cargo audit
```

- docs の導線が壊れていないこと
- `N02` canonical / `N05` optional non-commercial overlay の線引きを崩していないこと
- PostgreSQL / MySQL が primary write、SQLite が read-only artifact という前提を崩していないこと

## Dependency vulnerability checks

[OSV-Scanner v2](https://google.github.io/osv-scanner/installation/) を PATH に入れ、
この clone で pre-commit を有効にします。既に Git フックを使っている場合は、
下の設定を適用する前に既存のフックから `.githooks/pre-commit` を呼ぶように統合してください。

```bash
git config --local core.hooksPath .githooks
```

pre-commit はステージ済みの `Cargo.lock` と `frontend/package-lock.json` を検査します。
脆弱性の検出、ロックファイルの欠落、スキャンの失敗はいずれもコミットを止めます。
フックは clone ごとに有効化が必要です。GitHub では `OSV dependency gate` を
`main` の必須チェックに設定し、端末にかかわらずマージ時に検査します。

CI は同じロックファイルに加え、Dockerfile のベースイメージ、Compose の全プロファイル、
CI の DB サービス、ビルドした API・ops・crawler の各イメージを検査します。
重大度を問わず OSV が報告した脆弱性をすべてブロックします。
定期実行は毎日 09:45 JST が目安です。コンテナ公開はゲートの成功後に進み、
GHCR に push するイメージも公開の直前にスキャンします。

Dependabot は Cargo・npm・GitHub Actions のバージョンを週次、
Dockerfile と Compose のイメージを日次で確認し、更新 PR を作成します。
更新 PR にも同じ必須チェックが適用されます。複数の依存に脆弱性がある場合は、
修正を同一の PR にまとめ、検査対象がすべて通る状態でマージします。
