# ROADMAP

## 2026-04-20 時点の整理

PR #2 までで、この repo はもう空箱ではありません。
`station-ops job ingest-n02` による ingest、`validate-ingest`、SQLite export、`station-api`、
example frontend、self-hosted の運用導線まで揃っています。

一方で、次の価値は「機能をさらに増やすこと」より、
**利用者が安心して受け取って使える状態を仕上げること**にあります。

## 結論: 次はこの順番

1. Release と配布物の信頼性を完成させる
2. OpenAPI を production-ready にする
3. ingest 速度ではなく、検知 -> 公開速度を上げる
4. データ品質ゲートを product-grade にする
5. README から迷わず辿れる docs 導線を作る

## 1. Release と配布物の信頼性を完成させる

### なぜ最優先か

今の repo には verified tag `v0.1.0` があります。
それ自体は強いです。
ただし、利用者視点では「落として検証して使える release product」はまだ完成していません。

### いま出来ていること

- `./scripts/release_sqlite_artifact.sh <db>` で local bundle を作れる
- `./scripts/publish_sqlite_release.sh <db> <tag>` で GitHub Release へ upload できる導線がある
- CI でも SQLite artifact の smoke build は回している
- release bundle は次を生成する
  - `stations.sqlite3`
  - `manifest.json`
  - `SOURCE_METADATA.json`
  - `checksums.txt`
  - `CHANGELOG.md`
  - `RELEASE_NOTES.md`
  - `README_SQLITE.md`
  - `SBOM.spdx.json`
- `v*` tag push 用の GitHub Release workflow がある
- `stations.sqlite3` の provenance attestation と SBOM attestation に対応した

### 残る operational task

- 新しい patch tag を切って GitHub Release を publish する
- release asset を実際の public release page に載せる
- `gh attestation verify` を使う consumer-side verification 導線を README / docs からさらに短くする
- 必要なら `v0.1.0` を backfill するのではなく、`v0.1.x` の新 tag で前に進める

### 重要な運用メモ

現行の `scripts/publish_sqlite_release.sh` は **tag が現在の `HEAD` と一致すること**を要求します。
2026-04-20 時点では `v0.1.0` は現在の `main` より前の commit を向いています。

したがって安全な進め方は次のどちらかです。

- `v0.1.0` の commit に checkout して、その commit 由来の release asset を作る
- 今の `main` で release bundle hardening を入れたあと、新しい patch tag を切る

**verified な `v0.1.0` tag を載せ替えない**でください。
この repo では patch tag を増やす方が正しいです。

## 2. OpenAPI を production-ready にする

### いま出来ていること

- [`API_SPEC.md`](../API_SPEC.md) はある
- `station-api` には次の実 endpoint がある
  - `/health`
  - `/ready`
  - `/v1/dataset/status`
  - `/v1/stations/search`
  - `/v1/stations/nearby`
  - `/v1/lines/catalog`
  - `/v1/lines/{line_name}/stations`
  - `/v1/operators/{operator_name}/stations`

### まだ足りないこと

- Rust handler / response type から OpenAPI JSON を自動生成する
- `/openapi.json` を出す
- `/docs` に Swagger UI, Scalar, Redoc のいずれかを載せる
- response schema を Rust 型と同期させる
- example request / response を固定する
- error response 仕様を統一する
- versioning policy を明記する
- `API_SPEC.md` を generated OpenAPI と同じ契約面へ寄せる

### 次の API product gap

`/v1/dataset/status` は既にあります。
その次に価値が高いのは次です。

- `/v1/dataset/snapshots`
- `/v1/dataset/changes`

利用者が「いつのデータか」「何が変わったか」を API で追えるようにするのが狙いです。

## 3. ingest 速度ではなく、検知 -> 公開速度を上げる

### いま出来ていること

- README baseline では local PostgreSQL fresh ingest が `total_ms=2039`
- 以前の支配的ボトルネックだった `persist_ms` は `10091 -> 630` まで落ちている
- same snapshot の skip path もある

### ここで勘違いしない

今の主戦場は「2 秒を 1 秒に削ること」ではありません。
価値が大きいのは、source の更新検知から release / status 反映までを短くすることです。

### 追加したい流れ

`station-ops job refresh-n02` のような flow で、少なくとも次を扱えるようにする。

1. source index を取得する
2. `source_version` / `source_url` / `source_sha256` を比較する
3. unchanged なら素早く終了する
4. changed なら download / verify / parse / validate に進む
5. persist は transaction 内だけに閉じ込める
6. commit 後に SQLite export と release asset publish を行う
7. `dataset_revision` 単位で cache invalidation する

### freshness claim の文言

この project が言える latest は、
**latest available MLIT N02 snapshot** です。

real-time railway data ではありません。
README と docs はこの線を崩さずに書くこと。

## 4. データ品質ゲートを product-grade にする

### いま出来ていること

- `station-ops validate-ingest` がある
- `--json` と `--strict` がある
- 現在は少なくとも次を見ている
  - station count
  - line count
  - operator count
  - blank station / line / operator
  - out-of-range / suspicious coordinates
  - duplicate latest `station_uid`

### まだ足りないこと

- distinct station name threshold
- line / operator threshold を latest baseline に合わせて引き上げる
- `station_versions` と `source_snapshots` の参照整合性チェック
- `valid_from` / `valid_to` interval check
- PostgreSQL / MySQL / SQLite parity check
- artifact parity check
- reproducibility check

### strict acceptance criteria の候補

2026-04-19 の README baseline から見ると、まずは次を候補にする。

- `active_station_count >= 10000`
- `distinct_station_name_count >= 9000`
- `distinct_line_count >= 600`
- latitude / longitude の hard range check
- blank `station_name`, `line_name`, `operator_name` は 0
- duplicate `station_uid` in `stations_latest` は 0

`distinct_operator_count` の厳密な閾値は、最新の PostgreSQL / MySQL 実測を再採番してから固定する。
ここは雑に hard-code しない方がよいです。

### テスト / tool backlog

- golden fixture test
- snapshot replay test
- two-snapshot diff test
- PostgreSQL / MySQL / SQLite parity test
- API contract test
- artifact reproducibility test
- `cargo nextest`
- `cargo llvm-cov`
- `cargo deny`
- `cargo audit`

## 5. README から迷わず辿れる docs 導線を作る

### いま出来ていること

README から次へは辿れます。

- operations
- database
- release
- architecture
- deploy
- source policy

### まだ足りないこと

役割別の入口がありません。
次は README と docs に「I want to...」導線を作るのが効きます。

### docs backlog

- `docs/INDEX.md`
- `docs/QUICKSTART_SQLITE.md`
- `docs/QUICKSTART_API.md`
- `docs/DATA_FRESHNESS.md`
- `docs/DATA_LICENSE.md`
- `docs/DATA_QUALITY.md`
- `docs/API.md`
- `docs/OPENAPI.md`
- `docs/ARTIFACTS.md`
- `docs/OBSERVABILITY.md`
- `docs/REDIS_CACHE.md`
- `docs/FAQ.md`

特に `DATA_LICENSE.md` は優先度が高いです。
canonical source が `N02`、`N05` は optional non-commercial overlay という線を、
README からすぐ辿れる形にする必要があります。

## いまやらない方がいいこと

### 1. 三大クラウド全部を production-ready にしない

`deploy/` と `infra/` は今のところ skeleton として持っておくのが正しいです。
全部を同時に production resource 化すると、データ OSS ではなく infra 保守 repo になります。

### 2. Redis を主役にしない

Redis は cache only です。
正本は DB と snapshot history に置きます。

cache key の基本方針は:

- `endpoint + normalized_query + dataset_revision`

### 3. N05 overlay を急がない

N05 は魅力的ですが、non-commercial restriction を持ちます。
canonical export に急いで混ぜないこと。

## Milestones

### v0.1.x: usable release

- GitHub Release を完成させる
- SQLite artifact bundle を添付する
- manifest / checksum / source metadata を添付する
- README に download / verify path を載せる
- `docs/ARTIFACTS.md`
- `docs/DATA_LICENSE.md`

### v0.2.0: API contract

- OpenAPI
- `/docs`
- API contract tests
- unified error model
- API versioning policy
- TypeScript client generation

### v0.3.0: trust package

- artifact attestation
- SBOM
- OpenSSF Scorecard
- `cargo-deny`
- Dependabot
- branch protection docs
- release checklist

### v0.4.0: operations

- `/metrics`
- source freshness watcher
- dataset snapshots API
- dataset changes API
- cache invalidation by dataset revision
- systemd production hardening

### v0.5.0: distribution

- GitHub Pages docs
- demo frontend
- Docker image
- AWS reference deployment
- GCP / Azure は artifact publish skeleton を維持

## Value statement

この project の価値は、
「駅データが取れる」だけではありません。

公式ソースから再現可能に取り込み、
差分を残し、
検証可能な形で、
API と SQLite として配れること。

次に詰めるべきなのは派手な機能ではなく、
利用者が安心して使える証拠です。
