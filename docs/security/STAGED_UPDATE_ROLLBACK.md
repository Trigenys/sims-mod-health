# Staged update and rollback pipeline

## Scope

Issue #18 introduces a **single-artifact** mutation pipeline for already indexed `.package` and `.ts4script` files.

Bulk update is intentionally not exposed. The native module keeps `BULK_UPDATE_ENABLED = false`, and no bulk Tauri command is registered.

The pipeline is deliberately conservative because mutation safety is more important than update coverage.

## Transaction order

Every update follows this order:

1. validate target and source inputs;
2. create and verify a restore point;
3. create a persistent update transaction journal;
4. download only from an allowlisted HTTPS source into app-controlled staging;
5. compute SHA-256 and verify the expected SHA-256 when one is available;
6. stage the replacement outside the Mods folder;
7. resolve the current installed release set and check the dependency graph;
8. archive the previous artifact;
9. install the replacement through a same-directory temporary file;
10. validate target containment, file type and SHA-256;
11. retain the restore point for rollback.

The Mods folder is not mutated before the restore point is in the `ready` state.

## Restore points

Restore points live under the Tauri app-data directory:

```text
<app-data>/restore-points/<operation-key>/tree/<relative Mods path>
```

The local SQLite `restore_points` row stores the app-local backup location plus a manifest containing:

- target relative path;
- backup relative path;
- original SHA-256;
- original-existed flag.

The backup SHA-256 must match the original file before mutation can proceed.

## Staging containment

Downloads never use a source-controlled filename as a destination.

Each transaction receives a generated staging directory:

```text
<app-data>/update-staging/<transaction-id>/
```

The initial download is always `replacement.download`; after integrity checks it becomes `replacement.staged`.

The pipeline canonicalizes the staging directory and verifies that it remains beneath the application data root. User-supplied target paths must be relative, contain only normal path components and end in `.package` or `.ts4script`.

Existing target parents may not traverse symbolic links.

## Download allowlist

Automatic download is limited to official adapter-owned delivery hosts.

### GitHub Releases

Accepted source kind: `github_releases`

Allowed HTTPS hosts:

- `github.com`
- `objects.githubusercontent.com`
- `release-assets.githubusercontent.com`

### CurseForge

Accepted source kind: `curseforge`

Allowed HTTPS hosts:

- `forgecdn.net`
- subdomains of `forgecdn.net`

Every redirect is re-evaluated against the same source-specific allowlist. URLs containing credentials, non-HTTPS schemes or non-standard ports are rejected.

Query strings and fragments are removed before source URLs are persisted in the local mutation journal so signed/tokenized download URLs are not retained.

Downloads are capped at 256 MiB.

## Integrity evidence

The staged file always receives an observed SHA-256.

When the update source provides an expected SHA-256, the transaction stops before mutation unless the observed value matches exactly.

When an expected SHA-256 is unavailable, the observed SHA-256 is still persisted and is used to verify the staged install and protect rollback from overwriting a file that changed later.

The transaction event log records source validation, download, integrity, dependency check, archive, install, validation and rollback phases.

## Dependency safety

Disable/remove is not permitted from incomplete dependency evidence.

Before the old artifact is archived, the pipeline:

1. resolves all enabled package/script files using deterministic registry matches;
2. stops if any enabled file remains unresolved;
3. verifies that the requested current release is actually in the resolved installed set;
4. evaluates the current dependency/conflict graph;
5. substitutes the proposed replacement release ID;
6. evaluates the hypothetical post-update graph;
7. rejects the update if it introduces a new missing/outdated dependency or known incompatibility.

Relationship analysis is intentionally not split across batches. If the resolved installed set exceeds the registry API's safe 500-release relationship limit, mutation stops instead of producing a partial dependency graph.

Existing unrelated dependency findings do not block a replacement by themselves; only regressions introduced by the hypothetical replacement do.

## Interrupted updates

`update_transactions` is a persistent state journal.

On startup, any non-terminal transaction is marked `interrupted` and receives a `startup_recovery` event. The restore point is not deleted.

An interrupted transaction can therefore be rolled back after restart.

## Rollback

Rollback:

1. verifies the restore-point directory is still inside app data;
2. verifies the backup SHA-256 against the stored original SHA-256;
3. refuses to overwrite a target that changed independently after the update;
4. removes the staged installed artifact when safe;
5. restores the backup through a same-directory temporary file;
6. verifies the restored SHA-256;
7. marks both the update transaction and restore point as restored.

## Current limitations

This first proven pipeline updates one existing local file at a time.

It does not yet:

- extract archives into multiple target files;
- mutate unresolved mods;
- bulk update;
- auto-remove dependencies;
- accept arbitrary creator-site download hosts.

Archive extraction and broader mutation policy remain subject to the hardening work in #21.
