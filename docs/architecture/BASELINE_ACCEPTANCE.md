# Architecture Baseline Acceptance

**Issue:** #2  
**Status:** Accepted for implementation  
**Date:** 2026-09-25

This document records the verification performed before implementation issues are allowed to build on the architecture baseline.

## Acceptance matrix

| Criterion | Evidence | Result |
|---|---|---|
| Architecture is internally consistent and implementable | `docs/ARCHITECTURE.md` separates local and shared authority, defines Tauri/Rust desktop responsibilities, a modular registry backend and a staged resolution pipeline. ADR-0001 through ADR-0004 freeze the cross-cutting decisions. | Pass |
| Raw mod files remain local by default | Architecture §1–3, Threat Model “Privacy leakage”, ADR-0001. | Pass |
| Script inspection is explicitly non-executing | Threat Model “TS4Script execution” and ADR-0002 require archive/data inspection only and prohibit importing/executing embedded code. | Pass |
| `Unknown` remains distinct from `Broken` | Architecture §9, ADR-0003 and Source Policy “Staleness”. | Pass |
| Prohibited scraping and redistribution are excluded | Source Policy “Prohibited behavior” and ADR-0004. | Pass |
| Unresolved blockers have linked issues | No architecture blocker remains for Foundation work. Implementation uncertainty is represented by PERT/backlog issues #3–#22. | Pass |

## Consistency review

### Local vs cloud ownership

No conflict found.

Local state owns the user's files, local paths, diagnostics, scan cache, SQLite inventory, backups and restore points. Shared services own public registry metadata and evidence. Fingerprints are the intended bridge between the two domains.

### WebView vs native privileges

No conflict found.

The current Tauri capability file grants only the core default capability. Product-specific filesystem access is intended to be implemented as narrow Rust commands. This matches both the architecture and threat model.

### Domain model vs resolution strategy

No conflict found.

`LocalArtifact` remains device-local while `Artifact`, `ModRelease` and `Mod` represent shared canonical identities. The resolution cascade can therefore return exact or probabilistic mappings without changing local ownership.

### Health state vs provenance

No conflict found.

Compatibility is not a free-standing boolean. It is evidence scoped to a game patch/version range. The health engine is required to preserve `Unknown` when trustworthy evidence is unavailable.

### Source policy vs planned adapters

No conflict found.

The planned CurseForge and GitHub adapters are compatible with the source-order policy. Generalized scraping is not part of the architecture.

## Accepted constraints

These are deliberate constraints, not blockers:

- Windows is the first supported desktop platform.
- Tauri 2 + React/TypeScript + Rust is the desktop baseline.
- FastAPI/PostgreSQL is the planned registry baseline.
- Redis/background workers are deferred until ingestion volume demonstrates a need.
- automatic mutation/update is deferred until backup and rollback are proven.
- no user account is required for local scanning.

## Review triggers

A new ADR is required before changing any of the following:

- raw mod files are uploaded by default;
- broad WebView filesystem capability is introduced;
- script mods are executed during inspection;
- `Unknown` is removed or treated as `Broken`;
- a prohibited/non-consensual scraping source is introduced;
- the product begins redistributing creator files.

## Proof of Done evidence

- **Issue:** #2
- **Automated tests:** N/A — documentation/architecture-only issue; product CI still validates the repository baseline.
- **Manual verification:** Cross-document consistency review captured above.
- **Security/privacy check:** Completed against `docs/security/THREAT_MODEL.md`; no new runtime permission or data collection introduced.
- **Performance evidence:** N/A — no runtime behavior changed.
- **Visual evidence:** N/A — no UI behavior changed.
- **Docs updated:** ADR index, four Accepted ADRs and this acceptance record.
- **Known limitations / follow-ups:** Implementation details remain intentionally delegated to issues #3–#22.
