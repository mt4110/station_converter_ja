# Repository Governance

This document captures the minimum repository rules for keeping releases and
dependency maintenance predictable.

## Required Checks

Require the following checks before merging to `main`:

- `ci / rust`
- `ci / frontend`
- `ci / integration-postgres`
- `ci / integration-mysql`
- `security / cargo-audit`
- `security / cargo-deny`
- `container-images / build (api, worker/api/Dockerfile)`
- `container-images / build (ops, worker/ops/Dockerfile)`
- `terraform-fmt / fmt`

`openssf-scorecard` should stay enabled as a scheduled and manual signal. It is
useful for trust hardening, but a newly failing Scorecard result should be
triaged separately from a build or security regression.

## Release Tags

Do not move an existing release tag.

If a release asset, generated bundle, or release note no longer matches an
existing tag, keep the old tag in place and publish a new patch tag instead.
Release assets should be built from the commit pointed to by the release tag.

## Dependabot PRs

Prefer one modest maintenance PR over merging stale Dependabot PRs one by one
when several of them touch the same maintenance surface.

- Merge a Dependabot PR directly only when it is current, narrow, and green.
- Close stale Dependabot PRs once their changes are absorbed into a verified
  maintenance PR.
- Split risky updates into their own PR. DB-layer majors such as `sqlx` should
  include PostgreSQL, MySQL, SQLite export, validation, and API checks.
- Do not use Dependabot grouping as a reason to accept a major update whose
  blast radius is larger than the current maintenance task.

## Security Workflow Failures

Treat `security` failures as priority maintenance work.

- `cargo-audit` advisory failures should block release work unless the advisory
  is explicitly ignored with a documented reason in `deny.toml` or the workflow.
- `cargo-deny` advisory failures should block merge until the dependency is
  updated or a narrow, documented exception is added.
- License and source failures should be treated as policy failures, not flaky CI.
- Wildcard dependency warnings are allowed by current policy, but should not
  hide advisory, license, or source errors.

## Release Workflow Permissions

The release workflow needs the minimum permissions required to publish and
attest the SQLite artifact:

- `contents: write` to create or update the GitHub Release assets
- `id-token: write` for provenance attestation
- `attestations: write` for artifact attestations
- `artifact-metadata: write` for SBOM attestation metadata

Other workflows should keep `contents: read` unless they publish artifacts,
attestations, packages, or security events.
