# ADR-0001 — Offline-first local authority

**Status:** Accepted  
**Decision date:** 2026-09-25  
**Issue:** #2

## Context

Sims Mod Health must inspect potentially large local libraries containing `.package`, `.ts4script`, diagnostics and game-version files. Uploading these files to a remote service would create unnecessary bandwidth, privacy, reliability and trust costs.

The product still benefits from shared data such as known releases, fingerprints, compatibility reports, dependencies and creator metadata.

## Decision

The desktop application is the authority for **local installation state**.

Local-only data includes:

- raw mod and custom-content files;
- absolute local paths;
- game installation paths;
- saves, households and screenshots;
- raw diagnostics unless the user explicitly chooses to share them;
- restore points and local mutation state.

The cloud registry is the authority for **shared public metadata**, including:

- creators and canonical mod identities;
- releases and artifacts;
- public fingerprints;
- compatibility evidence;
- dependencies and known incompatibilities;
- source provenance;
- recommendation taxonomy.

The default registry request contains only the minimum technical metadata needed for resolution, such as hashes/fingerprints, normalized version information and explicitly permitted metadata.

## Consequences

### Positive

- scanning continues to work without an account;
- raw libraries do not need to cross the network;
- very large mod folders remain practical;
- privacy defaults are easier to explain and enforce;
- cloud outages do not erase the local inventory.

### Costs

- some resolution features require a hybrid local/cloud workflow;
- local SQLite persistence becomes a first-class component;
- the desktop must handle incremental scanning and cache invalidation itself.

## Guardrails

- introducing raw-file upload requires a new ADR and explicit user-consent model;
- telemetry must redact absolute local paths by default;
- diagnostics sharing, if added, must be previewable and opt-in.
