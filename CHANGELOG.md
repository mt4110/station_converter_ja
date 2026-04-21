# Changelog

このプロジェクトの主要な変更はここに記録します。

## Unreleased

### Added

- product-grade SQLite release bundle
  - `manifest.json`
  - `SOURCE_METADATA.json`
  - `checksums.txt`
  - `README_SQLITE.md`
  - `RELEASE_NOTES.md`
  - `SBOM.spdx.json`
- tag push 用の GitHub Release workflow
- `stations.sqlite3` の build provenance attestation
- `SBOM.spdx.json` の SBOM attestation

### Changed

- release packaging が source snapshot、row counts、git commit、tool version を記録するようになった
- release publish script が bundle 一式を GitHub Release asset として upload するようになった
- OpenAPI の dataset history / change detail description と `API_SPEC.md` の読み方を同期した
- README / release docs / generated SQLite artifact notes に consumer-side verification flow を追加した

## v0.1.0

- initial public release tag
- N02 ingest / diff / SQLite export / API / self-hosted 運用導線の初期土台
