# OPERATIONS

## Runtime map

この repo の運用線は次で固定です。

- resident service: `station-api`
- scheduled job: `station-ops job ingest-n02`
- optional chained job: `station-ops job ingest-n02 --export-sqlite`
- dev helper: `station-crawler -- --loop`
- debug one-shot: `station-crawler -- --once`

本番では `station-crawler` を常駐 worker として使いません。

## First production setup

### 1. primary write DB を用意する

- PostgreSQL または MySQL
- SQLite は配布 artifact 用で、primary write にしません

### 2. env を固める

repo root での運用なら `worker/ops/.env` と `worker/api/.env` を使えます。  
systemd 運用なら [`deploy/systemd/station-converter-ja.env.example`](../deploy/systemd/station-converter-ja.env.example)
を `/etc/station_converter_ja/station.env` にコピーして編集します。

### 3. migrate

```bash
cargo run -p station-ops -- migrate
```

### 4. 初回 ingest

```bash
cargo run -p station-ops -- job ingest-n02
```

SQLite artifact まで一気に更新したい場合:

```bash
cargo run -p station-ops -- job ingest-n02 --export-sqlite
```

### 5. API を常駐させる

```bash
cargo run -p station-api
```

## systemd runbook

`systemd` を使う場合は、repo に置いてある実ファイルをそのまま土台にできます。

### Install

```bash
sudo install -d /etc/station_converter_ja
sudo cp deploy/systemd/station-converter-ja.env.example /etc/station_converter_ja/station.env
sudo cp deploy/systemd/station-converter-ja-api.service /etc/systemd/system/
sudo cp deploy/systemd/station-converter-ja-ingest-n02.service /etc/systemd/system/
sudo cp deploy/systemd/station-converter-ja-ingest-n02.timer /etc/systemd/system/
sudo systemctl daemon-reload
```

`/etc/station_converter_ja/station.env` では最低限次を埋めます。

- `DATABASE_TYPE`
- `POSTGRES_DATABASE_URL` または `MYSQL_DATABASE_URL`
- `SQLITE_DATABASE_URL`
- `BIND_ADDR`

ingest のたびに SQLite export まで繋げたい場合は、
`STATION_INGEST_ARGS=--export-sqlite` を設定します。

### Enable

```bash
sudo systemctl enable --now station-converter-ja-api.service
sudo systemctl enable --now station-converter-ja-ingest-n02.timer
```

手動実行:

```bash
sudo systemctl start station-converter-ja-ingest-n02.service
```

### Logs

```bash
journalctl -u station-converter-ja-api.service -f
journalctl -u station-converter-ja-ingest-n02.service -f
```

## External scheduler contract

systemd 以外の scheduler を使う場合も、呼ぶコマンドは同じです。

```bash
cargo run -p station-ops -- job ingest-n02
```

artifact 連動が必要なら:

```bash
cargo run -p station-ops -- job ingest-n02 --export-sqlite
```

## Lock policy

- lock file は `JOB_LOCK_DIR` 配下に置く
- 既定値は `storage/locks`
- `ingest-n02.lock` は `station-ops job ingest-n02` と `station-crawler` が共用
- `export-sqlite.lock` は `station-ops export-sqlite` と `--export-sqlite` chain が共用
- one-shot job で lock が取れない場合、その起動は失敗として終了
- dev loop で lock が取れない場合、その周回は skip

この方針で、本番 scheduler、手動 one-shot、dev loop の責務を混ぜずに済みます。

## Routine operations

### Release an artifact

```bash
./scripts/release_sqlite_artifact.sh postgres
```

MySQL を primary write にしている場合:

```bash
./scripts/release_sqlite_artifact.sh mysql
```

### Verify before updating

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
```

### Update procedure

1. 新しいコードを配置する
2. `cargo run -p station-ops -- migrate`
3. `station-api` を再起動する
4. 必要なら `station-ops job ingest-n02 --export-sqlite` を手動実行する
5. scheduler を通常運転に戻す

systemd なら必要に応じて timer を一時停止してから更新します。

```bash
sudo systemctl stop station-converter-ja-ingest-n02.timer
sudo systemctl restart station-converter-ja-api.service
sudo systemctl start station-converter-ja-ingest-n02.service
sudo systemctl start station-converter-ja-ingest-n02.timer
```

## Failure handling

### lock busy

重複実行防止が効いているだけです。  
先行ジョブが終わるのを待つか、意図しない重複起動かを確認します。

### ingest failed

まずはログを確認します。

- upstream 取得失敗
- DB 接続失敗
- migration 未適用

原因を直したあと、one-shot job を再実行します。

```bash
cargo run -p station-ops -- job ingest-n02
```

### export failed

ingest 成功後に export だけ失敗した場合は、export を単独で再実行できます。

```bash
cargo run -p station-ops -- export-sqlite
```

### rollback / republish

`artifacts/sqlite/` には時刻付きの成果物が残るので、
必要なら直前の known-good artifact を再配布できます。
