# frontend

Minimal Next.js example frontend.

Pages:

- `/`
- `/examples/address-search`
- `/examples/station-search`
- `/examples/line-search`
- `/examples/operator-search`
- `/examples/nearby-search`

Address search note:

- `/examples/address-search` は `frontend/app/api/address-search/route.ts` の hand-written route を使います
- この route は国土地理院 Address Search を包む example helper で、`station-api` の OpenAPI / generated station SDK には含めません

API client:

- `npm run generate:station-sdk`
- `npm run verify:station-sdk`
- 既定では repo 内の `cargo run -q -p station-api -- --dump-openapi-json` から OpenAPI を取得します
- `STATION_API_OPENAPI_SOURCE` に URL か JSON file path を渡すと、生成元を差し替えられます
- repo root の `./scripts/verify_repo.sh` からも freshness check として実行されます
