# Data Freshness

## Public Claim

This project can claim freshness up to the latest available MLIT N02 snapshot that
has been ingested and published by this system.

It does not provide real-time railway data.

## Source Refresh Flow

Use `refresh-n02` when you want to check whether the configured N02 snapshot differs
from the latest ingested snapshot:

```bash
cargo run -p station-ops -- job refresh-n02 --check-only
```

If changed, ingest it:

```bash
cargo run -p station-ops -- job refresh-n02
```

If changed and you also want a SQLite artifact:

```bash
cargo run -p station-ops -- job refresh-n02 --export-sqlite
```

The command computes the source ZIP SHA-256 and skips parse / persist when the digest
matches the latest stored N02 snapshot.

## Cache Invalidation

Cache keys should include dataset revision material, at minimum:

```text
endpoint + normalized_query + latest_source_sha256
```

Redis remains cache only. Source of truth stays in the primary DB and snapshot history.

