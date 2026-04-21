# OPENAPI

## Purpose

この doc は `station-api` の OpenAPI first pass の結果と、
残る contract polish を整理するためのメモです。
「次セッションで何を作るか」ではなく、
**2026-04-21 時点で何が実装済みで、どこがまだ仕上げ途中か**を揃えることを目的にします。

## Current State

2026-04-21 時点で、OpenAPI first pass は完了しています。
現在は読み物としての同期を進め、generated OpenAPI に
dataset history / change detail の field-level description を追加します。

- `worker/api` の route annotation / DTO から `/openapi.json` を自動生成できる
- `/docs` で Swagger UI を公開している
- current public API は次を含む
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
- `cargo run -q -p station-api -- --dump-openapi-json` で同じ contract を local dump できる
- `frontend/scripts/generate-station-sdk.mjs` が同じ contract から TypeScript SDK / 型定義を再生成する
- `./scripts/verify_repo.sh` が SDK freshness を確認し、`worker/api` の contract test が `/openapi.json` と `/docs` を確認する

## What First Pass Solved

- Rust handler と DTO を source of truth に寄せた
- `Json<Value>` 中心だった response を typed DTO に置き換えた
- base error envelope を `error.code` / `error.message` に寄せた
- `/v1/dataset/status` だけでなく `/v1/dataset/snapshots` と `/v1/dataset/changes` も contract に載せた
- docs UI と generated frontend SDK の wiring を verify / CI に乗せた

## Contract Boundary

machine-readable source of truth は `worker/api` にあります。

- `worker/api/src/main.rs`
- `worker/api/src/schema.rs`
- `worker/api/src/error.rs`
- `worker/api/src/openapi.rs`

補助 docs の役割は次のとおりです。

- [`API_SPEC.md`](../API_SPEC.md): 利用者向けの読み物 / example 集
- この doc: 実装境界と残タスクの整理
- generated SDK: frontend consumer 向けの typed client

`/api/address-search` はここに含めません。
`frontend/app/api/address-search/route.ts` の Next.js helper route で、
国土地理院 Address Search を使う example frontend 専用導線です。
`station-api` の public contract ではないため、
OpenAPI / generated station SDK の対象外に置く判断を維持します。

## Reading Dataset History

`/v1/dataset/status`、`/v1/dataset/snapshots`、`/v1/dataset/changes` は同じ N02 snapshot history を別の粒度で見ます。

- `/v1/dataset/status`: 今 API / sample frontend が使う `stations_latest` の概要
- `/v1/dataset/snapshots`: 取り込んだ source snapshot の履歴。newest first
- `/v1/dataset/changes`: station identity 単位の change events。newest first

`snapshots.items[].id` は `changes?snapshot_id=...` に渡せます。
`station_version_count` と `change_counts` は N02 station identity に scope されます。
`downloaded_at` と `created_at` は active database dialect が返す timestamp string で、
v1 では RFC3339 正規化していません。
この API が言える freshness は latest available MLIT N02 snapshot であり、
real-time railway data ではありません。

`changes.items[].detail` は consumer が field-level diff を読むための payload です。
現在の detail payload では、`updated` が `before` / `after` と `changed_fields` を持ちます。
`created` / `removed` は flat context fields を持つため、top-level context fields も fallback として扱います。
一覧 UI は top-level の `station_name` / `line_name` / `operator_name` だけでも表示できます。

## Design Decisions That Stay

### Library Choice

- `utoipa`
- `utoipa-swagger-ui`

理由:

- axum route と contract を近くに置ける
- Rust 型から schema を引ける
- `/docs` を小さな追加で導入できる

Swagger UI は引き続き first choice でよいです。
Scalar / Redoc 比較は contract 自体が十分安定してからで構いません。

### DTO Placement

public API DTO は `worker/api` に閉じます。

- `station_shared` は domain / DB 共通責務を保つ
- OpenAPI 都合を shared crate へ広げすぎない
- API 境界の変更点を `worker/api` で追いやすくする

### Operational Endpoint Semantics

- `/ready` は `200` / `503` とも同 shape を維持する
- readiness failure を generic error envelope に潰さない
- additive change は `/v1`、breaking change は `/v2`

## Remaining Polish

### 1. `API_SPEC.md` の同期

- example、文言、error note を generated contract と合わせ続ける
- `/openapi.json` と `/docs` が canonical reference であることを docs 側でも明確にする
- generated OpenAPI の field description と `API_SPEC.md` の説明を同じ意味に保つ

### 2. snapshots / changes の説明強化

- `/v1/dataset/snapshots` と `/v1/dataset/changes` 自体はもう public endpoint である
- 説明は「source snapshot の履歴」と「station identity の差分イベント」を混ぜない
- examples は count 固定に寄りすぎず、shape と読み方を示す

### 3. error detail 標準化

- 現状の共通 envelope は `error.code` / `error.message`
- field-level あるいは machine-readable detail が必要なら、non-breaking に optional `detail` を足す
- ここは OpenAPI と [`API_SPEC.md`](../API_SPEC.md) を同時更新する
- optional `detail` はまだ追加しない。まず current envelope を明文化する

### 4. hand-written endpoint 境界の明記

- frontend-local helper である `/api/address-search` は generated contract に含めない
- README / API docs でその境界を短く書いて、`station-api` と example frontend の責務を混ぜない

### 5. contributor path の固定

- public API change 時は generated OpenAPI、`API_SPEC.md`、SDK generation を同じ patch で更新する
- verify path は `cargo test` と `./scripts/verify_repo.sh` を維持する

## Next Execution Order

1. `API_SPEC.md` と README の文言を generated contract に合わせ続ける
2. snapshots / changes の example を count 固定に寄りすぎない形で整える
3. `ApiErrorResponseDto` に optional `detail` が必要かを、実需要が出た時点で決める
4. `generate:station-sdk` / `verify:station-sdk` / CI wiring を green のまま保つ

## Done Enough

- `/openapi.json` と `/docs` が current public API を反映し続ける
- Rust DTO / example / generated SDK が同期している
- [`API_SPEC.md`](../API_SPEC.md) が読み物として機能しつつ、別契約にならない
- frontend-only helper endpoint が public `station-api` contract の外だと docs で分かる
