# ROADMAP

## 2026-04-21 時点の整理

PR #2 までで、この repo はもう空箱ではありません。
`station-ops job ingest-n02` による ingest、`validate-ingest`、SQLite export、`station-api`、
example frontend、self-hosted の運用導線まで揃っています。

一方で、次の価値は「機能をさらに増やすこと」より、
**利用者が安心して受け取って使える状態を仕上げること**にあります。

## 結論: 次はこの順番

1. Release と配布物の信頼性を完成させる
2. OpenAPI contract の仕上げをする
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

## 2. OpenAPI contract の仕上げをする

### いま出来ていること

- `worker/api` の route / DTO から OpenAPI JSON を自動生成できる
- `/openapi.json` と `/docs` を公開している
- `station-api` の current public API が contract に載っている
  - `/health`
  - `/ready`
  - `/v1/dataset/status`
  - `/v1/dataset/snapshots`
  - `/v1/dataset/changes`
  - `/v1/stations/search`
  - `/v1/stations/nearby`
  - `/v1/lines/catalog`
  - `/v1/lines/{line_name}/stations`
  - `/v1/operators/{operator_name}/stations`
- `frontend/scripts/generate-station-sdk.mjs` で同じ contract から TypeScript SDK / 型定義を再生成できる
- `./scripts/verify_repo.sh` と `worker/api` の contract test で `/openapi.json` / `/docs` / 主要 path を確認している

### 現在の polish 状態

- [x] [`API_SPEC.md`](../API_SPEC.md) を generated OpenAPI と同期し、human-readable companion として位置づける
- [x] `/v1/dataset/snapshots` と `/v1/dataset/changes` の利用者向け説明 / sample を整える
- [x] `/api/address-search` のような frontend-local helper が OpenAPI 外であることを docs と contract test に固定する
- [x] versioning policy と public API change 時の更新手順を contributor docs に固定する
- [x] error response は `code` / `message` に加え、optional `detail.kind` / `detail.issues[]` の標準形を持つ

### 次の contract gap

OpenAPI 自体を「入れること」は終わりました。
ここから効くのは endpoint 追加より、**contract を読み違えない状態に寄せること**です。

- snapshots / changes を見れば「いつのデータか」「何が変わったか」が追える
- だから次は、その shape と error semantics をぶらさない方が価値が高い

## 3. ingest 速度ではなく、検知 -> 公開速度を上げる

### いま出来ていること

- README baseline では local PostgreSQL fresh ingest が `total_ms=2039`
- 以前の支配的ボトルネックだった `persist_ms` は `10091 -> 630` まで落ちている
- same snapshot の skip path もある
- `station-ops job refresh-n02 --check-only` で configured N02 source の SHA-256 を比較できる
- `station-ops job refresh-n02 --export-sqlite` で changed source の ingest と SQLite export を繋げられる

### ここで勘違いしない

今の主戦場は「2 秒を 1 秒に削ること」ではありません。
価値が大きいのは、source の更新検知から release / status 反映までを短くすることです。

### 追加したい流れ

`station-ops job refresh-n02` は次を扱います。

1. configured source ZIP を取得する
2. `source_sha256` を最新 ingested N02 snapshot と比較する
3. unchanged なら parse / persist へ進まず終了する
4. changed なら download / verify / parse / persist に進む
5. persist は transaction 内だけに閉じ込める
6. commit 後に SQLite export まで繋げられる

今後の追加余地:

- upstream index discovery
- release asset publish までの自動 chain
- `dataset_revision` 単位で cache invalidation する

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

### 現在の強化状態

- [x] distinct station name threshold
- [x] line threshold を latest baseline に合わせて引き上げる
- [x] `station_versions` と `source_snapshots` / `station_identities` の参照整合性チェック
- [x] `valid_from` / `valid_to` interval check
- [x] operator threshold を latest baseline に合わせて引き上げる
- [x] primary DB / SQLite artifact count parity check
- [x] latest source digest parity check
- [x] logical SQLite artifact reproducibility check

### strict acceptance criteria の候補

2026-04-19 の README baseline から見ると、まずは次を候補にする。

- `active_station_count >= 10000`
- `distinct_station_name_count >= 9000`
- `distinct_line_count >= 600`
- latitude / longitude の hard range check
- blank `station_name`, `line_name`, `operator_name` は 0
- duplicate `station_uid` in `stations_latest` は 0

`distinct_operator_count` は 2026-04-19 の PostgreSQL / MySQL baseline から、
default floor を `>= 170` にしています。

### テスト / tool backlog

- golden fixture test
- snapshot replay test
- two-snapshot diff test
- PostgreSQL / MySQL / SQLite parity test
- API contract test
- [x] artifact reproducibility test
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

### 現在の状態

README と docs に「I want to...」導線が入り、役割別入口を持っています。

### docs backlog

- [x] `docs/INDEX.md`
- [x] `docs/QUICKSTART_SQLITE.md`
- [x] `docs/QUICKSTART_API.md`
- [x] `docs/DATA_FRESHNESS.md`
- [x] `docs/SOURCE_POLICY.md` (current source / license entrypoint)
- [x] `docs/DATA_QUALITY.md`
- [x] `docs/API.md`
- [x] `docs/OPENAPI.md`
- [x] `docs/ARTIFACTS.md`
- [x] `docs/OBSERVABILITY.md`
- [x] `docs/REDIS_CACHE.md`
- [x] `docs/FAQ.md`

特に source / license policy は優先度が高いです。
現状は [`docs/SOURCE_POLICY.md`](./SOURCE_POLICY.md) がその役割を担っています。
canonical source が `N02`、`N05` は optional non-commercial overlay という線を、
README からすぐ辿れる形に保ちます。

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

### v0.2.0: API contract first pass

- [x] OpenAPI
- [x] `/docs`
- [x] API contract tests
- [x] base error envelope
- [x] TypeScript client generation

### v0.2.x: contract polish

- [x] `API_SPEC.md` sync
- [x] snapshots / changes docs
- [x] error detail standardization
- [x] hand-written frontend helper docs
- [x] contributor / PR update path

### v0.3.0: trust package

- artifact attestation
- SBOM
- OpenSSF Scorecard
- `cargo-deny`
- Dependabot
- branch protection docs
- [x] release checklist

### v0.4.0: operations

- `/metrics`
- [x] source freshness watcher
- [x] publish pipeline
- cache invalidation by dataset revision policy
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
