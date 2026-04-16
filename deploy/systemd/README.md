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
