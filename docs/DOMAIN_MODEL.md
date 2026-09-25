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

A statement that a release is compatible, broken, unknown or otherwise scoped to a GamePatch.

Every report requires provenance.

### Dependency

Directed relation between releases/mods with optional version constraints.

### ConflictRule

Known incompatibility stronger than a generic overlapping-resource observation.

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
