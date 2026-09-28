# ADR-0005 — Unified Game, DLC and Mod health boundary

**Status:** Accepted  
**Issue:** #72

## Context

Sims Mod Health began as an offline-first health manager for The Sims 4 mods and custom content. The product now needs to reason about the installed game build and installed content packs because those states directly affect mod compatibility.

A game patch can make a previously healthy mod incompatible, while a content pack can require a minimum game build. Treating Game, DLC and Mods as unrelated products would duplicate detection, update and recovery workflows and would make the desktop interface harder to understand.

Existing production boundaries already provide:

- a Rust/Tauri privileged filesystem boundary;
- local game-version discovery;
- SQLite persistence;
- an evidence-first health model;
- source adapters and provenance;
- staged mod mutation with restore points and rollback;
- a five-destination desktop information architecture.

Prior art from Anadius-style and successor Sims 4 updaters demonstrates useful concepts such as sentinel-file version fingerprints, manifest-driven planning, pack minimum-version metadata and explicit update state machines. Those concepts are architectural references only unless a dependency has a separately verified reuse license.

## Decision

### 1. One health product, not three managers

Game, DLC and Mods are health domains inside Sims Mod Health. They do not become separate primary products.

The primary navigation remains exactly:

```text
Overview
Library
Health
Discover
Settings
```

Game and DLC health are exposed through progressive disclosure in existing surfaces.

### 2. Hexagonal core

Game/DLC support uses Ports & Adapters around the existing Rust core.

Initial ports:

- `GameVersionProbe` — obtains game-build evidence without guessing.
- `PackInventoryProbe` — inventories locally present content packs.
- `ContentManifestSource` — supplies provenance-bearing public game/pack metadata.
- `PlatformUpdateAdapter` — hands an update action to a supported official provider.
- `ContentHealthRepository` — persists normalized local observations and update workflow state.

Adapters are replaceable and must not leak provider-specific schemas into domain or UI types.

### 3. Supporting patterns

- **Strategy** selects provider-specific behavior such as EA App, Steam or Unknown/manual.
- **State Machine** owns update lifecycle transitions.
- **Repository** keeps SQLite persistence behind domain-oriented operations.
- **Anti-Corruption Layer** normalizes external manifests/provider observations into Sims Mod Health domain types.
- **Command boundary** keeps privileged filesystem/process work behind narrow Tauri commands.

### 4. Evidence and ownership separation

Local files may prove that a pack is present. They do not prove that the account owns or is entitled to that pack.

The domain must therefore keep separate concepts for:

- local presence;
- integrity evidence;
- compatibility evidence;
- provider identity;
- entitlement/ownership, which Sims Mod Health does not infer.

### 5. Update behavior

For production Game/DLC updates, the default architecture is **official-provider handoff**:

```text
Detected
  -> ActionRequired
  -> ProviderOpened
  -> AwaitingRescan
  -> Verified | StillOutdated | Unknown | Failed
```

The app does not claim success until local evidence confirms the resulting build/pack state.

Direct binary patching is a separately gated capability. A delta engine may only be enabled when the payload source, redistribution/use rights, provenance and integrity chain are independently proven. It is not required for Game/DLC Health.

### 6. Reuse policy

- Anadius/Toasty-style updater implementations are **prior art** for architecture unless their code license is independently verified.
- `jmacd/xdelta` is a candidate reusable implementation for VCDIFF/Xdelta under its applicable Apache-2.0 licensing, but production use remains gated by payload-source review.
- Existing Sims Mod Health Rust scanner, fingerprint, SQLite and mutation/recovery modules remain authoritative instead of importing another manager runtime.

### 7. UI contract

- **Overview** shows compact game build, pack count, mod count and top-priority health state.
- **Health > Updates** is the single update queue for Game, Packs and Mods.
- Healthy packs collapse into a summary rather than one card per pack.
- Pack details open contextually; there is no Pack/DLC primary page.
- **Settings** owns detected provider/path/configuration only.
- **Library** remains the local mod/CC inventory unless a later ADR deliberately broadens it.

## Consequences

### Positive

- patch changes can trigger immediate mod compatibility reevaluation;
- provider-specific behavior stays replaceable;
- the UI gains capability without gaining navigation complexity;
- external updater schemas cannot silently become our domain model;
- local evidence remains useful offline.

### Costs

- Game/DLC metadata needs its own provenance and staleness handling;
- provider handoff is less seamless than a direct patcher;
- pack inventory and game-install discovery require additional Windows-specific probes.

## Rejected alternatives

### Separate Game / DLC / Updater navigation

Rejected because it duplicates the same health evidence and turns a focused utility into a launcher-style suite.

### Embed a third-party updater wholesale

Rejected because it would duplicate our Rust scanner/storage/recovery boundaries, introduce unclear licensing risk, and couple product behavior to another project's schema and distribution assumptions.

### Infer ownership from installed pack folders

Rejected because filesystem presence is not entitlement evidence.

## Follow-up

Implementation is tracked by #73, #74, #75 and #76. Direct delta-patching research is isolated in #77.
