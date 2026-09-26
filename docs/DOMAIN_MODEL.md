# Domain Model

## Core concepts

### GamePatch

Represents a specific The Sims 4 game version or supported version range.

Important fields:

- normalized version
- platform
- release date when known
- source/provenance

### Creator

Canonical creator identity plus aliases and source accounts.

### Mod

Logical product independent of a particular release or filename.

A Mod owns:

- name
- creator
- aliases
- description
- categories
- features
- canonical sources

### ModRelease

A versioned release of a Mod.

A release may declare:

- version
- release date
- game compatibility
- changelog
- dependencies
- known incompatibilities
- one or more downloadable artifacts

### Artifact

A concrete downloadable/installable file.

Examples:

- `.package`
- `.ts4script`
- creator-provided archive

An Artifact may have several fingerprints.

### Fingerprint

Technical identity used to resolve a local file to a known artifact.

Kinds may include:

- SHA-256
- CurseForge fingerprint
- normalized resource signature

### CompatibilityReport

A sourced statement about one release's health on either:

- one exact `GamePatch`; or
- an inclusive patch-version range with optional open minimum/maximum bounds.

Every report requires provenance. Exact patch evidence is more specific than range evidence. If current evidence from different sources disagrees at the same specificity, the health engine surfaces `Unknown` with a disputed flag instead of inventing a source winner.

Update availability is derived from release ordering and remains separate from compatibility evidence.

### DependencyRule

A provenance-bearing required relation from one release to either a canonical target Mod or a source identity that has not yet been canonicalized.

The target may carry minimum/maximum version constraints. Installation analysis distinguishes missing, outdated and version-mismatch states.

### ConflictRule

A provenance-bearing known incompatibility from one release to a target Mod/source identity with optional target-version constraints.

A known incompatibility is stronger evidence than generic DBPF resource overlap. Resource overlap remains a local `Potential conflict` observation and never becomes a confirmed incompatibility by itself.

### LocalArtifact

Device-local record representing one scanned file. It never needs to exist in the shared registry.

## Identity model

A local filename is not a canonical Mod identity.

Resolution should prefer:

1. exact cryptographic/source fingerprint;
2. trusted embedded metadata;
3. stable resource signature;
4. normalized filename + creator alias;
5. probabilistic candidate matching.

The UI must retain the evidence level so the user can distinguish exact identity from a likely guess.
