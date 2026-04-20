# INGEST HARDENING PLAN

## Purpose

次セッションで、駅データ ingest の信頼性と性能を実装可能な粒度で前に進めるための設計書兼プラン。

対象は `worker/crawler` と `worker/ops` を中心にした ingest path。
frontend や API の機能追加はこのフェーズでは主目的にしない。

## Current Facts

- canonical source は MLIT / 国土数値情報 `N02`
- 取得単位は「全国 ZIP 1本」であり、鉄道会社ごとの個別収集ではない
- 現状は
  - async download / async file write はある
  - ZIP 展開は同期
  - GeoJSON parse は同期
  - DB 差分反映は 1 transaction 内で diff-aware に処理する
  - identity / version / change_event persist は chunked batch 化済み
  - chunk size は env で比較可能
- 差分の整合性は
  - source SHA-256
  - source snapshot uniqueness
  - version history
  - change events
  で守っている

## Measured Baseline

2026-04-18 時点のローカル確認:

- source ZIP size: 約 12.7 MB
- parsed_features: 10,235
- parsed_stations: 10,155
- distinct operators: 178
- distinct lines: 552
- distinct station names: 8,504
- local ZIP -> empty SQLite initial ingest: 約 2.33 秒
- same snapshot re-ingest with skip: 約 1.8 秒

この数値は local SQLite での参考値であり、PostgreSQL / MySQL の本番相当値を保証するものではない。

2026-04-19 時点のローカル PostgreSQL 実測:

- source: MLIT `N02-24`
- parsed_features: `10,235`
- parsed_stations: `10,155`
- `validate-ingest`: `ok`
- bulk persistence 導入前:
  - fresh PostgreSQL initial ingest: `persist_ms=10091`, `total_ms=12044`
- bulk persistence 導入後:
  - fresh PostgreSQL initial ingest:
    - `load_ms=582`
    - `save_zip_ms=1`
    - `extract_ms=21`
    - `parse_ms=308`
    - `diff_ms=28`
    - `persist_ms=630`
    - `total_ms=2039`
  - same snapshot re-ingest with skip:
    - `persist_ms=4`
    - `total_ms=952`

同じ local ZIP を使った write chunk size sweep:

- `INGEST_WRITE_CHUNK_SIZE=200`: `persist_ms=658`, `total_ms=1498`
- `INGEST_WRITE_CHUNK_SIZE=500`: `persist_ms=626`, `total_ms=1460`
- `INGEST_WRITE_CHUNK_SIZE=1000`: `persist_ms=596`, `total_ms=1422`

この比較では PostgreSQL は `1000` が最良だった。

2026-04-19 時点のローカル MySQL 実測:

- same local ZIP + fresh MySQL initial ingest:
  - `INGEST_WRITE_CHUNK_SIZE=200`: `persist_ms=574`, `total_ms=1384`
  - `INGEST_WRITE_CHUNK_SIZE=500`: `persist_ms=627`, `total_ms=1455`
  - `INGEST_WRITE_CHUNK_SIZE=1000`: `persist_ms=632`, `total_ms=1465`
- `validate-ingest` は `warning` で通過
  - warning 理由は `source_url` が local file (`file://...`) だったため

この比較では MySQL は `200` が最良だったため、env 未指定時の default write chunk size は DB ごとに分ける:

- PostgreSQL / SQLite: `1000`
- MySQL: `200`

補足:

- MySQL の default collation (`utf8mb4_0900_ai_ci`) では `COUNT(DISTINCT line_name)` が bytewise distinct より 1 件少なく見えるケースがあった
- `validate-ingest` の distinct line / operator count は bytewise semantics に寄せて cross-DB で揃える
- API の `stations/search` / `lines/{line_name}/stations` / `operators/{operator_name}/stations` も MySQL では binary collation に寄せ、`の` と `ノ` のような別値が混ざらないようにする
- これらは local machine の reference 値であり、別マシンや別ストレージ条件での最適値を保証するものではない

## Problems To Solve

1. upstream 由来のデータを ingest 後にどこまで信用してよいかの受け入れ基準が弱い
2. ingest のどこが遅いかを示す phase timing が不足している
3. DB 書き込みは逐次 round-trip が多く、データ量増加や将来 overlay に弱い
4. TUI / local operator から見ると「成功したが本当に中身は正しいのか」が分かりにくい

## Goals

- ingest 後の acceptance check を CLI で明示的に回せる
- ingest report だけで最低限の品質と所要時間が読める
- bulk 化の前に、現在のボトルネックを数値で比較できる
- 既存の diff-aware design を壊さずに高速化できる

## Non-Goals

- 個社公式サイトとのリアルタイム照合
- N05 overlay の本実装
- GIS スタックの導入
- 先に並列 upsert ありきで複雑な実装に飛び込むこと

## Design Principles

- まず correctness を固める
- 次に measurement を入れる
- その後に bulk 化する
- 最適化は report で効果が見える形にする
- 失敗時は中途半端に commit しない

## Workstream A: Post-Ingest Acceptance Checks

### New CLI

`worker/ops` に新しい subcommand を追加する。

```text
station-ops validate-ingest
```

想定オプション:

- `--json`: machine-readable output
- `--strict`: warning を failure 扱いにする
- `--min-stations <N>`
- `--min-lines <N>`
- `--min-operators <N>`

### Validation Rules

初期フェーズで入れる検査:

1. `stations_latest` count が閾値以上
2. distinct `line_name` が閾値以上
3. distinct `operator_name` が閾値以上
4. `station_name`, `line_name`, `operator_name` の空文字 / NULL が 0
5. `latitude`, `longitude` が日本の想定範囲外に出ていない
6. `stations_latest` に duplicate `station_uid` がない
7. 最新 snapshot が存在する
8. `created + updated + unchanged + removed` の説明可能性を report で確認できる

### Output Shape

`--json` 時の例:

```json
{
  "status": "ok",
  "snapshot_id": 12,
  "checks": [
    { "name": "min_station_count", "status": "ok", "observed": 10155, "expected": ">=10000" },
    { "name": "blank_station_name", "status": "ok", "observed": 0, "expected": 0 }
  ]
}
```

### Failure Policy

- hard failure:
  - DB query failure
  - latest snapshot missing
  - station count / line count / operator count below floor
  - duplicate latest `station_uid`
- warning:
  - source URL missing
  - source URL が local file
  - representative point suspicious but not invalid

## Workstream B: Ingest Phase Timing

`IngestReport` に phase timing を追加する。

追加候補:

- `load_ms`
- `save_zip_ms`
- `extract_ms`
- `parse_ms`
- `diff_ms`
- `persist_ms`
- `total_ms`

これにより、次のセッションで

- download が重いのか
- unzip / parse が重いのか
- DB persist が重いのか

を分離して見られるようにする。

## Workstream C: Bulk Persistence

### Current Bottleneck

現状は station ごとに

- identity sync
- latest version compare
- version insert
- change event insert

を順番に実行している。

設計は読みやすいが、DB round-trip が多い。

### Phase 1 Bulk Strategy

いきなり並列 upsert には行かない。
先に次の低リスク改善を狙う。

1. latest versions は現行通り先読み
2. in-memory diff result を `created / updated / unchanged / removed` に分離
3. `created` と `updated(after)` の version insert を batch 化
4. `change_events` insert を batch 化
5. identity upsert を batch 化

実装候補:

- `sqlx::QueryBuilder` を使った chunked multi-row insert
- chunk size は 200-1000 程度で比較

実装メモ:

- `INGEST_WRITE_CHUNK_SIZE` で identity / version / change_event batch size を切り替えられる
- `INGEST_CLOSE_CHUNK_SIZE` で stale version close update の chunk size を切り替えられる
- env 未指定時の default は PostgreSQL / SQLite が `1000`、MySQL が `200`

### Why Not Parallel Upsert First

- 1 transaction と diff integrity を維持したい
- `station_uid` ごとの順序性を崩す必要がない
- まず batch 化だけで十分に速くなる可能性が高い
- 並列化はエラー制御と rollback の複雑さが大きい

### Future Optional Optimization

PostgreSQL 専用でさらに詰める場合:

- temp staging table
- bulk load into staging
- SQL で merge

ただしこれは cross-DB symmetry を崩すので、次セッションの first pass ではやらない。

## Workstream D: Local Operator Flow

local で中身確認する operator flow は次を標準にする。

1. `Prepare Env + DB`
2. `Migrate`
3. `Ingest N02`
4. `validate-ingest`
5. `API`
6. `Sample Web`
7. `DB Web UI`

補足:

- `Crawler Loop` は dev helper であり、operator が日常的に使う main path は `station-ops job ingest-n02`
- TUI config の crawler loop command は現行 CLI に合わせて修正済み
- TUI の `Quick Start` は `validate-ingest` を正式ステップに含み、直近の validation 結果と実行時刻を一覧 / 詳細で見られる
- TUI では validate mode を `standard / strict` で切り替えられ、実行中 workflow の cancel もできる
- TUI の `Quick Start` 実行中は current step と elapsed time を右ペインで見られる

## Test Plan

### Already Added

- duplicate segment grouping
- representative point midpoint
- identical snapshot idempotency
- created / updated / removed diff tracking
- `stations/search` の `の` / `ノ` bytewise semantics regression
- `lines/{line_name}` / `operators/{operator_name}` / `lines/catalog` の cross-DB search semantics regression
- `validate-ingest` happy path on in-memory SQLite
- `validate-ingest` fails on low-count fixture
- `validate-ingest` fails on blank names
- `validate-ingest` fails on duplicate latest station_uid
- `IngestReport` phase timing fields are populated and monotonic enough for sanity

### Next Tests To Add

- none right now

## Implementation Order

### Phase 0

- keep canonical source as N02 only
- do not add N05 in this session

### Phase 1

- add `station-ops validate-ingest`
- add validation queries and JSON output
- add CLI exit code policy
- add tests for validation command

Done condition:

- local operator can run ingest and validation separately
- CI can fail when acceptance floor is broken

### Phase 2

- instrument `IngestReport` timings
- print timings in CLI logs
- document baseline before optimization

Done condition:

- report includes parse / diff / persist cost
- a before/after performance comparison is possible

### Phase 3

- refactor persist step into explicit diff result collections
- batch insert identities / versions / change events
- compare throughput against Phase 2 baseline

Done condition:

- no behavioral regression in diff tests
- measured improvement on local SQLite and local PostgreSQL

### Phase 4

- optionally surface validation summary in TUI or API ready check

Done condition:

- operator can see pass/fail without manual SQL

Current note:

- TUI では `validate-ingest` の pass / warning / fail を見られるようになった
- TUI では `Quick Start` の strict validate を切り替えられる
- TUI では実行中 workflow を cancel できる
- API ready check への surfacing は未着手

## Suggested Session Kickoff Checklist

次セッション開始直後にやること:

1. read this file
2. implement `station-ops validate-ingest`
3. add validation tests before changing performance code
4. run local ingest + validate on PostgreSQL
5. only then start batch persistence refactor

## Success Criteria

- upstream snapshot を ingest したあと、機械的に「使ってよいか」を判定できる
- report に timing が出て、最適化前後を比較できる
- batch 化後も diff-aware semantics が保たれる
- local operator が TUI + DB Web + sample web で確認しやすい
