# Source Policy

## Goal

The registry must be useful without becoming a scraper that ignores creator rules or redistributes content it does not own.

## Preferred source order

1. official structured API
2. creator-published machine-readable manifest
3. public GitHub release metadata
4. creator page explicitly permitting automated access
5. community report with provenance and moderation

## Initial adapters

### CurseForge

Use the official API for projects, files, fingerprints, game versions, dependencies and changelogs where permitted by the API contract.

### GitHub Releases

Use public repository/release metadata for creators who publish releases on GitHub.

### Creator manifest

Sims Mod Health proposes an optional `simsmod.json` contract so creators can publish canonical identity, version, compatibility and dependency metadata.

Example:

```json
{
  "schemaVersion": 1,
  "id": "creator.mod-name",
  "name": "Example Mod",
  "version": "4.2.1",
  "gameVersions": ["1.128"],
  "categories": ["relationships"],
  "dependencies": [],
  "download": "https://example.invalid/releases/4.2.1"
}
```

## Prohibited behavior

Do not:

- scrape a source whose terms prohibit bots/scraping;
- bypass authentication, paywalls or rate limits;
- redistribute mod files without permission;
- mirror paid or creator-gated content;
- treat community mirrors as canonical without verification;
- remove creator attribution.

## Provenance

Every registry statement that can affect health should retain:

- source type
- source URL/identifier
- retrieval timestamp
- observed version/release
- confidence or trust level
- superseding evidence when updated

## Staleness

Source failure must produce a stale/unknown state, not a fabricated compatibility conclusion.

## Takedown and correction

The data model should support:

- correcting an identity mapping;
- withdrawing a bad fingerprint;
- marking a source unavailable;
- honoring creator requests concerning metadata where legally/operationally appropriate;
- preserving enough audit history to explain prior health decisions.
