# Documentation Index

## Start Here

- Run the API locally: [QUICKSTART_API.md](./QUICKSTART_API.md)
- Inspect the SQLite artifact: [QUICKSTART_SQLITE.md](./QUICKSTART_SQLITE.md)
- Understand API shape: [API.md](./API.md), [OPENAPI.md](./OPENAPI.md), [../API_SPEC.md](../API_SPEC.md)
- Understand source, license, and freshness: [SOURCE_POLICY.md](./SOURCE_POLICY.md), [DATA_LICENSE.md](./DATA_LICENSE.md), [DATA_FRESHNESS.md](./DATA_FRESHNESS.md)
- Check data quality: [DATA_QUALITY.md](./DATA_QUALITY.md)
- Ship a release: [RELEASE.md](./RELEASE.md), [RELEASE_CHECKLIST.md](./RELEASE_CHECKLIST.md), [ARTIFACTS.md](./ARTIFACTS.md)
- Operate a service: [OPERATIONS.md](./OPERATIONS.md), [OBSERVABILITY.md](./OBSERVABILITY.md)
- Use Redis safely: [REDIS_CACHE.md](./REDIS_CACHE.md)
- Answer common questions: [FAQ.md](./FAQ.md)

## Product Boundaries

- Canonical source is MLIT / National Land Numerical Information `N02`.
- `N05` is optional non-commercial overlay only.
- PostgreSQL / MySQL are primary write DBs.
- SQLite is a read-only artifact.
- Redis is cache only.
- Public freshness means latest available MLIT N02 snapshot, not real-time railway data.
