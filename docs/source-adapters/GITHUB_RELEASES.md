# GitHub Releases source adapter

## Purpose

The GitHub Releases adapter ingests public repository, release, tag and release-asset metadata into the shared registry without downloading or redistributing mod files.

## Access modes

The adapter supports two server-side modes:

- unauthenticated public GitHub API access;
- optional authenticated access through `GITHUB_RELEASES_TOKEN` to improve API rate limits.

The token is never required from end users and is never included in errors or stored registry metadata.

Even when a server token can see private repositories, the adapter rejects repositories whose GitHub `private` flag is true. Private or gated release content is therefore outside this adapter's ingestion boundary.

## Provenance

A repository is stored as a `github_releases` source using the stable GitHub repository ID as the external source identity.

For each public release the registry retains:

- GitHub release ID and tag;
- release URL, publication timestamp and changelog body;
- prerelease state and target commitish;
- each release asset ID, filename, size, MIME type and public download URL;
- GitHub-provided asset digest when available, normalized to the registry's canonical `sha256-v1` identity;
- observed repository tags and commit SHAs;
- retrieval timestamp.

Release assets are referenced by their public GitHub URLs. The adapter does not download or mirror them.

## Failure semantics

- HTTP 404 becomes `not_found`;
- authenticated visibility of a private repository becomes `not_public`;
- transport failures, GitHub rate limits and retryable server errors become `unavailable`;
- rate-limit retry timing uses `Retry-After` or `X-RateLimit-Reset` when provided.

Source failures do not create compatibility claims.

## Configuration

Optional environment variables:

- `GITHUB_RELEASES_TOKEN`
- `GITHUB_RELEASES_BASE_URL` for tests or controlled mirrors; production defaults to `https://api.github.com`.
