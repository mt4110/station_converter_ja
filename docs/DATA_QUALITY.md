# Data Quality

## Validation Command

```bash
cargo run -p station-ops -- validate-ingest
cargo run -p station-ops -- validate-ingest --strict --json
```

The standard acceptance floor is:

- `active_station_count >= 10000`
- `distinct_station_name_count >= 9000`
- `distinct_line_count >= 600`
- `distinct_operator_count >= 170`
- blank station / line / operator names are zero
- hard out-of-range coordinates are zero
- duplicate latest `station_uid` rows are zero

## Integrity Checks

`validate-ingest` also checks:

- `station_versions.snapshot_id` resolves to `source_snapshots.id`
- `station_versions.station_uid` resolves to `station_identities.station_uid`
- `valid_to` is either null or later than `valid_from`
- latest snapshot change summary is explainable by created / updated / removed rows

## SQLite Parity

After export:

```bash
cargo run -p station-ops -- verify-sqlite-parity
```

This compares primary DB counts and latest source digest against `SQLITE_DATABASE_URL`.

Logical reproducibility of two consecutive exports can be checked with:

```bash
./scripts/verify_sqlite_reproducibility.sh
```
