# ARCHITECTURE

## Runtime pieces

- `worker/crawler`
  - upstream 監視
  - snapshot download
  - normalize
  - diff event generation
- `worker/api`
  - station search API
  - line search API
  - nearby search API
- `worker/ops`
  - migrations
  - export / packaging
- `frontend`
  - sample integration UI
- `infra/terraform`
  - cloud deployment skeleton
- `deploy`
  - future k8s / helm / argocd path

## Data model

```text
source_snapshots -> station_versions -> stations_latest(view)
                  -> station_change_events
station_identities -> station_versions
```

## Non-goals in v1 scaffold

- full GIS stack
- PostGIS-first schema
- direct SQLite write path from crawler
- all clouds production-ready on day one
