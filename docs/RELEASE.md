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
./scripts/publish_sqlite_release.sh postgres v0.1.1
```

MySQL を primary write にしている場合は次です。

```bash
./scripts/publish_sqlite_release.sh mysql v0.1.1
```

この script は次をまとめて行います。

- `station-ops export-sqlite` で SQLite を再生成
- `scripts/package_sqlite_release.sh` で時刻付き bundle を作成
- 既存 tag に紐づく GitHub Release を作成または再利用
- `stations-*.sqlite3`, `checksums-*.txt`, `manifest-*.txt` を upload

前提:

- `gh` CLI が入っていること
- `gh auth login` 済みであること
- 指定する tag が **現在の HEAD と同じ commit** を向いていること
- tag が remote に push 済みであること

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

## Outputs

`artifacts/sqlite/` に次を生成します。

- `stations-<timestamp>.sqlite3`
- `checksums-<timestamp>.txt`
- `manifest-<timestamp>.txt`

manifest には生成時刻、SHA-256、サイズを入れます。
publish 後は、同じ 3 点が GitHub Release asset にも並びます。

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
