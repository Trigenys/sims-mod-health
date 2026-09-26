# Local SQLite Database

## Purpose

The local database persists device-local state required by scans, diagnostics and future rollback operations.

It is **not** a cache of the shared cloud registry and it does not define canonical creator/mod identities.

## Location

At runtime the desktop initializes:

```text
<Tauri app data directory>/sims-mod-health.sqlite3
```

Only the installation roots need absolute paths. Files inside the Mods tree are stored as paths relative to the installation's `mods_root` where possible.

## Schema ownership

### installations

Local Sims installations and their roots/version observations.

### scan_sessions

Scan history, mode, lifecycle and aggregate counters.

### local_files

One local filesystem entry per installation-relative path.

Migration v2 adds the incremental cache inputs:

- `size_bytes`
- `modified_ns`
- `quick_fingerprint`
- `hashed_at`

The primary fast cache key is:

```text
installation_id + relative_path + size_bytes + modified_ns
```

A later scanner issue decides when this cache key is sufficient and when a full hash must be recomputed.

### local_artifacts

Parsed technical metadata for a local file, independent of a canonical cloud identity.

### fingerprints

Locally computed SHA-256, CurseForge, quick, DBPF resource-signature or TS4Script identity-signature fingerprints.

### conflict_observations

Evidence produced by local scans. Resource overlap remains an observation rather than an automatic `Broken` conclusion.

### diagnostics

Parsed local diagnostic observations and redacted summaries. The schema does not require raw diagnostic bodies to be persisted.

### restore_points

Verified backup metadata for recoverable mutation/update operations.

### update_transactions

Persistent staged-update journal. It records the restore point, relative target, sanitized source provenance, current/replacement release IDs, expected/observed/original SHA-256 values, lifecycle state and recoverable failure information.

### update_events

Append-only local evidence for update phases such as restore-point creation, source validation, integrity verification, dependency checks, archive/install, startup recovery and rollback.

### preferences

Small local application preferences stored as JSON values.

## Migration policy

Migrations are versioned SQL files embedded into the Rust binary and tracked with SQLite `PRAGMA user_version`.

Current versions:

1. initial local persistence model;
2. incremental-scan cache metadata and index;
3. recoverable scan observations tied to scan sessions;
4. persisted TS4Script identity fingerprints while preserving existing fingerprint rows;
5. persisted incremental-scan skipped/observation counters for real Overview state;
6. staged-update transaction and event journals linked to restore points.

Migrations are **forward-only**.

The application never attempts an automatic destructive down-migration. If a database reports a schema version newer than the running application supports, startup fails that storage initialization rather than silently modifying the database.

A destructive schema correction must be shipped as a new forward migration with an explicit data-preservation plan.

## Connection policy

Each opened database enables:

- foreign-key enforcement;
- a 5-second busy timeout;
- WAL journal mode for the on-disk database.

## Privacy boundary

This database is device-local.

It may contain user-specific paths because installation discovery needs them locally. Those paths are not part of the registry API contract and must be redacted from telemetry.

The schema intentionally contains no required cloud account, creator, canonical mod or remote source table.

## Verification

Rust tests cover:

- migration from zero to the latest schema;
- forward migration with data preservation, including fingerprint schema v4;
- presence of the incremental cache index;
- foreign-key enforcement;
- refusal to downgrade a future schema version.
