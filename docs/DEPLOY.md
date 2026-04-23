# DEPLOY

## Supported today

この repo でそのまま実運用に乗せやすい導線は **self-hosted + external scheduler / systemd** です。

- resident API: `station-api`
- scheduled ingest: `station-ops job ingest-n02`
- optional SQLite artifact chain: `--export-sqlite`

`deploy/systemd/` に実ファイルを置いています。

container image で運用する場合も役割は同じです。
API と one-shot ops image の tag policy / 最小起動例は
[`docs/CONTAINER_IMAGES.md`](./CONTAINER_IMAGES.md) にまとめています。

## Cloud skeletons kept in-tree

次は将来拡張用の骨格として残しています。

- `deploy/helm/station-converter-ja/`
- `deploy/k8s/base/`
- `deploy/argocd/`
- `infra/terraform/`

現時点では、これらは「位置と責務の約束」を表すもので、
production resource 実装まではまだ入れていません。

## Artifact publishing targets

SQLite artifact の公開先としては次を想定しています。

- AWS: S3
- GCP: Cloud Storage
- Azure: Blob Storage

artifact 自体の生成導線は [`docs/RELEASE.md`](./RELEASE.md) にまとめています。
