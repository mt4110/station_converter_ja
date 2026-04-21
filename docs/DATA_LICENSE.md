# DATA_LICENSE

This page is the consumer-facing license entrypoint for distributed data artifacts.
For source selection rules and internal policy, see [SOURCE_POLICY.md](./SOURCE_POLICY.md).

## Canonical Artifact

The canonical SQLite release artifact is built from MLIT / 国土数値情報 `N02`.

The public freshness claim is limited to the latest available MLIT N02 snapshot
included in the release. This is not real-time railway data.

## N02 License Posture

N02 is the v1 canonical source because recent snapshots are suitable for broad
distribution, including commercial use, under the source terms described by MLIT.

Release metadata is included in:

- `manifest.json`
- `SOURCE_METADATA.json`
- `checksums.txt`
- `SBOM.spdx.json`

Consumers should verify the downloaded release assets before use.

## N05 Overlay

`N05` is not part of the canonical SQLite artifact.

It may be useful for historical open / close / rename tracking, but it carries
non-commercial restrictions. Treat it as an optional overlay only, and do not
silently mix it into canonical export.

## Redistribution Notes

- Keep MLIT / 国土数値情報 `N02` attribution with redistributed artifacts.
- Preserve `SOURCE_METADATA.json` when repackaging release data.
- Do not describe the artifact as real-time railway data.
- Do not treat SQLite as the primary write database.
