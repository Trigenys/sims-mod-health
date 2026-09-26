# Overview health aggregation

## Purpose

The Overview is a triage surface built from the latest local scan plus the registry state engine. It does not use the design-target fixture in normal desktop runtime.

## Data flow

1. The Tauri backend selects the most recently seen Sims installation.
2. SQLite provides the latest scan session and current local file inventory.
3. Enabled package/script artifacts are sent to the registry matching endpoint in bounded batches using supported fingerprints.
4. Resolved canonical release IDs are evaluated by the patch compatibility state engine.
5. The dependency/conflict endpoint evaluates missing or outdated dependencies and known incompatibilities.
6. Exact duplicate and DBPF resource-overlap evidence stays local.
7. The backend aggregates the result into one Overview snapshot for the React UI.

Only the local filename, file size, identity hints and supported fingerprints are sent for artifact resolution. Local relative paths and DBPF resource-key overlap details stay on-device.

## Overall health

The score is **verified patch-compatibility coverage**, not a severity score.

```text
verified compatible installed releases
-------------------------------------- × 100
resolved installed releases + unresolved enabled files
```

A release with an update available still contributes to the numerator when its installed release is currently compatible. Update maintenance and compatibility are intentionally separate concepts.

Unknown or unresolved items remain in the denominator because the app cannot claim them as healthy.

When registry health is unavailable, the score is shown as unavailable rather than guessed from local filenames or absence of reports.

## Summary cards

The four cards preserve the design hierarchy:

- **Healthy** — resolved installed releases whose final state is `compatible`.
- **Updates** — resolved installed releases with `update_available`.
- **Conflicts** — conflict/risk findings from health state, known incompatibilities, exact duplicate groups and DBPF resource overlaps.
- **Unknown** — current-patch `unknown` releases plus enabled files that did not resolve to a canonical release.

These cards are triage counters, not a partition of every file. For example, an update can still contribute to the overall health numerator when its installed release is compatible.

## Current installation counts

The installation panel uses only local SQLite data:

- **items indexed** — all current local files tracked for the installation;
- **script mods** — enabled `.ts4script` files;
- **package / CC files** — enabled `.package` files;
- **unidentified files** — enabled files not resolved to a canonical registry release;
- **exact duplicate groups** — groups sharing the same current SHA-256 fingerprint.

“Package / CC files” is deliberately file-level wording. A package-only gameplay mod is not silently reclassified as CC.

## Needs attention ordering

Findings are deterministic and conservative:

1. broken current-patch evidence;
2. abandoned releases and known incompatibilities;
3. missing/outdated dependencies and exact duplicates;
4. potential conflicts and disputed evidence;
5. updates;
6. unresolved/unknown items.

Resource overlap is always labelled **Potential conflict**. It is never promoted to guaranteed breakage.

## Registry states

- **ready** — artifact resolution, compatibility and relationship analysis are available;
- **partial** — one stage is unavailable or the local scan has recoverable observations;
- **offline** — the registry cannot be reached, while local scan/duplicate counts remain available.

Offline mode never fabricates compatibility. Health score and unresolved-registry counts may be unavailable until connectivity returns.

## Scan freshness and progress

The latest scan is marked stale when its completed timestamp is more than 24 hours old.

The desktop listens to `scanner://progress` and surfaces:

- files seen;
- files hashed;
- unchanged/skipped files;
- recoverable observations.

After a scan completes, Overview reloads from SQLite and then refreshes registry health.

## Registry endpoint configuration

The desktop resolves the registry base URL in this order:

1. runtime environment variable `SIMS_MOD_HEALTH_REGISTRY_URL`;
2. local preference key `registry.base_url`;
3. development fallback `http://127.0.0.1:8000`.

This keeps endpoint selection configurable without hard-coding a production service into the UI.
