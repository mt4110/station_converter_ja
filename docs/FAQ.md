# FAQ

## Is this real-time railway data?

No. Freshness is limited to the latest available MLIT N02 snapshot ingested by this project.

## Is SQLite the primary database?

No. PostgreSQL / MySQL are primary write DBs. SQLite is a read-only artifact.

## Can N05 be mixed into the canonical export?

No. N05 is optional non-commercial overlay only. Do not silently mix it into canonical N02 export.

## Why does `/api/address-search` not appear in OpenAPI?

It is a Next.js helper route for the example frontend. It is not part of public `station-api`.

## What should I run before publishing?

```bash
./scripts/verify_repo.sh
./scripts/verify_ingest_export.sh postgres
./scripts/verify_ingest_export.sh mysql
cargo run -p station-ops -- validate-ingest --strict --json
```

Then follow [RELEASE_CHECKLIST.md](./RELEASE_CHECKLIST.md).

