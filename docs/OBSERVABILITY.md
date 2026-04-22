# Observability

## Current Signals

- `station-ops job ingest-n02` logs source URL, SHA-256, snapshot id, row counts,
  diff counts, and phase timings.
- `station-ops job refresh-n02` logs whether the configured source changed and
  whether ingest ran.
- `station-ops validate-ingest --json` emits machine-readable quality checks.
- `station-ops verify-sqlite-parity` emits machine-readable artifact parity checks.
- `/ready` reports database readiness, cache mode, and a lightweight dataset readiness summary.
- `/metrics` exposes Prometheus text-format gauges for API/database scrape status
  and active canonical N02 dataset counts.

## Recommended Operator Checks

```bash
cargo run -p station-ops -- job refresh-n02 --check-only
cargo run -p station-ops -- validate-ingest --strict --json
cargo run -p station-ops -- verify-sqlite-parity
curl http://localhost:3212/v1/dataset/status
curl http://localhost:3212/metrics
```

## Not Yet Implemented

- structured release dashboard
- external alert routing
