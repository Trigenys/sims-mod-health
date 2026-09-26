# Dependency and conflict analysis

## Purpose

Issue #15 separates four different kinds of installation risk instead of collapsing them into one generic "conflict" label:

1. exact duplicate files;
2. overlapping DBPF resources;
3. required dependency problems;
4. known incompatibility rules.

Those categories have different evidence strength and therefore different wording and actions.

## Local evidence

The desktop owns file-level evidence because raw package structure stays local.

### Exact duplicate

Exact duplicates are grouped only from the current SHA-256 fingerprint.

This is deterministic byte identity. Two files with the same filename but different SHA-256 values are not duplicates.

### Resource overlap

The Tauri command `analyze_local_conflicts` parses enabled `.package` files through the bounded read-only DBPF parser and compares live Type/Group/Instance resource keys.

A shared resource key is returned as:

`Potential conflict`

It is not treated as proof that either package is broken. Overrides are a normal Sims modding mechanism and may be intentional.

The analyzer returns:

- both local file IDs and relative paths;
- number of shared resources;
- a bounded sample of resource keys;
- parser failures without aborting the rest of the analysis;
- a truncation flag if more than 10,000 file pairs overlap.

Exact duplicates and resource overlaps are separate result collections even when the same pair appears in both.

## Registry relationship rules

The shared registry stores two provenance-bearing rule types.

### DependencyRule

A release may require either:

- a canonical target Mod; or
- a source identity such as `curseforge:77` when the target has not yet been unified into a canonical Mod.

Optional minimum and maximum target versions constrain the dependency.

### ConflictRule

A release may be known incompatible with a canonical Mod or source identity, again with optional version constraints.

Every rule stores:

- source ID;
- source URL;
- source record ID when available;
- retrieval timestamp;
- notes.

CurseForge `required_dependency` and `incompatible` file relations are normalized into these rule tables without inventing version constraints that CurseForge did not provide.

## Installation relationship API

Endpoint:

```text
POST /v1/health/relationships/evaluate
```

Input is a bounded list of resolved installed release IDs.

The response exposes:

- missing dependencies;
- outdated dependencies;
- dependency version mismatches;
- known incompatibilities whose target version constraint actually matches;
- reverse dependency usage;
- dependency cycles.

### Reverse usage

For every installed dependency that is referenced by another installed release, the response includes:

- dependency release ID;
- `used_by_count`;
- exact release IDs that depend on it.

This is the safety primitive used before future disable/remove operations. A shared dependency is therefore visible as "used by N installed mods" rather than being removed blindly.

### Cycles

Dependency cycles are detected as strongly connected components. Cycles do not recurse indefinitely and do not prevent the rest of the installation from being analyzed.

## Version behavior

A missing target is actionable as `install_dependency`.

When an installed target is below a minimum version, it is actionable as `update_dependency`.

An unknown/unparseable version or a target above an explicit maximum is returned as a version mismatch requiring review.

Known incompatibility is only emitted when the installed target satisfies the rule's target version range. If the target version is unknown and the rule is version-constrained, the engine does not claim an incompatibility.

## Privacy boundary

Local resource keys and local paths are not sent to the registry relationship endpoint.

The registry receives only canonical installed release IDs for dependency and known-incompatibility analysis. File-level duplicate/resource analysis remains on-device.
