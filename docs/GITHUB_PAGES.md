# GitHub Pages

GitHub Pages should be a small public entrypoint, not a second product site.

## Purpose

Use Pages to help a reader quickly answer:

- What artifact can I download?
- How do I verify the SQLite artifact?
- How fresh is the data?
- What is canonical `N02`, and why is `N05` not mixed into the canonical export?
- How do I run the API or inspect the example frontend locally?

## Minimum Page Set

The first Pages version should link to existing docs rather than duplicate them:

- Project overview: [`../README.md`](../README.md)
- SQLite artifact quickstart: [`QUICKSTART_SQLITE.md`](./QUICKSTART_SQLITE.md)
- Release artifacts and verification: [`ARTIFACTS.md`](./ARTIFACTS.md)
- API quickstart: [`QUICKSTART_API.md`](./QUICKSTART_API.md)
- API contract: [`API.md`](./API.md), [`OPENAPI.md`](./OPENAPI.md), [`../API_SPEC.md`](../API_SPEC.md)
- Source, license, and freshness: [`SOURCE_POLICY.md`](./SOURCE_POLICY.md), [`DATA_LICENSE.md`](./DATA_LICENSE.md), [`DATA_FRESHNESS.md`](./DATA_FRESHNESS.md)
- FAQ: [`FAQ.md`](./FAQ.md)

## Publishing Boundary

Do not claim real-time railway freshness on Pages. The public freshness claim
remains latest available MLIT N02 snapshot.

Do not present `N05` as part of the canonical artifact. It remains optional,
non-commercial overlay material only.

## Recommended Setup

Start with GitHub Pages publishing Markdown from the repository docs entrypoint.
After the dependency-update PR is green, add a dedicated Pages workflow or Pages
configuration in a separate PR so Pages deployment failures do not obscure
dependency-update CI.
