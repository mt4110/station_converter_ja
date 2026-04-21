# Quickstart: API

## Start

```bash
./scripts/setup_nix_docker.sh postgres
nix develop
cargo run -p station-ops -- migrate
cargo run -p station-ops -- job ingest-n02
cargo run -p station-api
```

Default API port is `3212`.

## Check

```bash
curl http://localhost:3212/health
curl http://localhost:3212/ready
curl http://localhost:3212/v1/dataset/status
```

OpenAPI:

- JSON: `http://localhost:3212/openapi.json`
- Docs UI: `http://localhost:3212/docs`

## Example Queries

```bash
curl 'http://localhost:3212/v1/stations/search?q=新宿&limit=5'
curl 'http://localhost:3212/v1/lines/catalog?q=中央&limit=20'
curl 'http://localhost:3212/v1/dataset/snapshots?limit=5'
curl 'http://localhost:3212/v1/dataset/changes?limit=10'
```

For endpoint semantics, see [API.md](./API.md) and [../API_SPEC.md](../API_SPEC.md).

