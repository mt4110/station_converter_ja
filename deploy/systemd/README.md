# deploy/systemd

Production-ready reference units for the supported self-hosted path.

- `station-converter-ja-api.service`
  - resident `station-api`
- `station-converter-ja-ingest-n02.service`
  - one-shot `station-ops job ingest-n02`
- `station-converter-ja-ingest-n02.timer`
  - daily scheduler example
- `station-converter-ja.env.example`
  - shared environment file template

Copy the unit files into `/etc/systemd/system/`, copy the env example to
`/etc/station_converter_ja/station.env`, then edit the DB URLs and paths for your host.
The reference units expect a `station-converter-ja` system user and group.
They also expect these binaries to exist:

- `/opt/station_converter_ja/target/release/station-api`
- `/opt/station_converter_ja/target/release/station-ops`

From the repository root, you can build and stage them with:

```bash
sudo ./scripts/install_release_binaries.sh /opt/station_converter_ja station-converter-ja station-converter-ja
```

See `docs/OPERATIONS.md` for the full install and update runbook.
