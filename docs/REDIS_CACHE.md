# Redis Cache

Redis is optional and cache only.

Do not store source of truth in Redis. The source of truth is the primary DB plus
`source_snapshots`, `station_versions`, and `station_change_events`.

## Key Shape

Use dataset revision material in every cache key:

```text
station-api:v1:{endpoint}:{normalized_query}:{latest_source_sha256}
```

When a new N02 snapshot is ingested, keys with the old source SHA naturally fall out
of use. Explicit deletion is optional unless storage pressure requires it.

## Readiness

`READY_REQUIRE_CACHE=true` can make `/ready` treat Redis as required. Keep it false
when Redis is only a performance optimization.

