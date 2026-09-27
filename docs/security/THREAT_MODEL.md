# Threat Model

## Scope

Sims Mod Health parses third-party files, inspects archives and may later download updates. The application must assume mod content is untrusted even when the creator is reputable.

## Assets

Protect:

- the user's filesystem
- saves and game installation
- local Sims data
- authentication credentials
- registry integrity
- update/rollback state
- creator/source identity
- the user's privacy

## Trust boundaries

```mermaid
flowchart LR
  Mods[Untrusted mod files] --> Native[Rust native parser boundary]
  Native --> Local[(SQLite)]
  Native --> UI[WebView UI]
  Native --> API[Registry API]
  Sources[External sources] --> API
  API --> DB[(Registry DB)]
  API --> Native
```

The WebView does not receive unrestricted filesystem access.

## Primary threats and controls

### Malformed DBPF

Threat: crafted resource tables, huge counts, invalid offsets, integer overflow or excessive allocation.

Controls:
- validate offsets against file size;
- checked integer arithmetic;
- cap resource count and individual read size;
- stream where practical;
- reject malformed structures cleanly;
- maintain adversarial fixtures and fuzz targets.

Current DBPF parser limits:
- at most 1,000,000 declared resource entries;
- at most 128 MiB declared index bytes;
- fixed 96-byte header read;
- scalar 2/4-byte index reads only;
- zero resource-payload bytes read during metadata parsing;
- unknown index-layout flag bits are rejected rather than guessed.

### TS4Script execution

Threat: executing bundled Python or native payload while trying to inspect it.

Controls:
- treat `.ts4script` as data/archive only;
- never import or execute embedded modules;
- inspect names/metadata with bounded reads;
- do not shell out to Python to load the mod.

Current TS4Script inspector limits:
- physical archive: 256 MiB;
- archive entries: 4,096;
- single expanded entry: 64 MiB;
- total declared expanded bytes: 512 MiB;
- maximum compression ratio: 500:1;
- path depth: 32 components;
- entry-name length: 1,024 bytes;
- individual metadata read: 64 KiB;
- total metadata reads: 256 KiB;
- nested archives are never recursively opened.

### Archive bombs

Threat: extreme expansion ratio or deeply nested archive content.

Controls:
- maximum archive entries;
- maximum total expanded metadata budget;
- nesting limits;
- cancellation;
- no automatic extraction into user-controlled paths.

### Path traversal

Threat: archive member or update package writes outside a staging directory or Mods target.

Controls:
- normalize and validate every destination;
- reject parent traversal, dot components, absolute paths and unsupported target extensions;
- generated staging destinations never reuse remote filenames;
- canonical staging directories must remain under app data;
- existing Mods target parents may not traverse symlinks;
- stage before install;
- prefer Rust-native file operations.

### Untrusted downloads

Threat: compromised mirror, redirect abuse or malicious replacement.

Controls:
- source-adapter-specific HTTPS host allowlists;
- every redirect is revalidated against the same allowlist;
- credential-bearing URLs and non-standard ports are rejected;
- signed/tokenized query strings are not persisted in mutation logs;
- source provenance is journaled;
- expected SHA-256 verification when available;
- observed SHA-256 is always recorded and rechecked at install time;
- 256 MiB download limit;
- generated staging filenames under app-controlled storage;
- verified restore point before download/mutation.

### Registry poisoning

Threat: false compatibility, creator or release data.

Controls:
- provenance per statement;
- source trust levels;
- immutable evidence history where feasible;
- distinguish creator/API facts from community reports;
- conflict resolution rules;
- audit trail for moderator changes.

### False-positive conflict claims

Threat: overlapping resource keys are presented as guaranteed breakage.

Controls:
- state model uses `Potential conflict`;
- deterministic `Broken` requires stronger evidence;
- expose why a conflict was flagged.

### Privacy leakage

Threat: sending filenames, saves, usernames, local paths or mod content unnecessarily.

Controls:
- raw files stay local by default;
- API requests prefer hashes/fingerprints and normalized technical metadata;
- redact local absolute paths from telemetry;
- diagnostics upload is opt-in and previewable;
- no saves/households/screenshots are collected for registry resolution.

### Excessive Tauri permissions

Threat: compromised WebView gains broad filesystem access.

Controls:
- default capability remains minimal;
- privileged work happens in narrow Rust commands;
- every new capability requires threat-model review and Proof of Done evidence;
- CI audits capability drift and rejects broad filesystem/shell/process/HTTP permissions.

Current capability:
- `core:default` only.

### WebView content injection

Threat: injected markup or compromised local content executes script, frames the app or opens a direct WebView network path.

Controls:
- production Content Security Policy is explicit;
- scripts are self-only;
- `unsafe-eval` is forbidden;
- object embedding and framing are disabled;
- WebView network access is limited to Tauri IPC;
- registry/source networking remains in native Rust.

Accepted residual risk:
- `style-src 'unsafe-inline'` remains enabled because runtime UI progress/score rendering uses inline style attributes. This does not grant inline-script execution.

### Diagnostic telemetry without consent

Threat: user-specific diagnostic metadata is transmitted without an affirmative privacy choice.

Controls:
- diagnostic telemetry preference defaults to false;
- malformed preference values fail closed;
- opt-in and revocation are persisted locally;
- the current product has no automatic raw diagnostic upload;
- telemetry preview is redacted before it can be eligible for sharing.

### Dependency supply chain

Threat: a known vulnerable JavaScript, Python or Rust dependency is shipped into beta.

Controls:
- Security CI audits npm, pip and Cargo dependency graphs;
- known high/critical findings block the beta security gate;
- dependency audit results are part of security-sensitive Proof of Done.

## Review status

The release-gate threat/evidence matrix, accepted residual risks and verification commands are maintained in `docs/security/HARDENING_REVIEW.md`.

## Non-goals

The first releases do not attempt to:

- execute mods safely in a sandbox;
- prove that arbitrary gameplay behavior is semantically correct;
- crack, decompile or bypass paid mod access;
- download from sources that prohibit automated retrieval.

## Security reporting

Do not file sensitive vulnerability details in a public issue. Until a dedicated private disclosure channel exists, report security findings directly to the repository owners through the organization's private communication channel.
