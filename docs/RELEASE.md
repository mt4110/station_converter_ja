# RELEASE

## Goal

SQLite artifact を「export しただけ」で終わらせず、配布物としてまとめるための導線です。

## Official path

1. primary write DB を最新化する
2. `station-ops export-sqlite` で SQLite を再生成する
3. `scripts/package_sqlite_release.sh` で配布物を固める

普段はこれをまとめた次のコマンドを使います。

```bash
./scripts/release_sqlite_artifact.sh postgres
```

MySQL を primary write にしている場合は次です。

```bash
./scripts/release_sqlite_artifact.sh mysql
```

## Outputs

`artifacts/sqlite/` に次を生成します。

- `stations-<timestamp>.sqlite3`
- `checksums-<timestamp>.txt`
- `manifest-<timestamp>.txt`

manifest には生成時刻、SHA-256、サイズを入れます。

## CI / local verification

主要経路の確認は次で揃います。

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
```

`verify_ingest_export.sh` は repo 内の小さな N02 fixture を ZIP 化して使うので、
外部 upstream に依存せず `migrate -> ingest -> export` を再現できます。
