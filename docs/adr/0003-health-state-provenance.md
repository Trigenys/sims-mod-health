# ADR-0003 — Evidence-based health states and provenance

**Status:** Accepted  
**Decision date:** 2026-09-25  
**Issue:** #2

## Context

The Sims 4 receives frequent patches, while mod creators publish updates at different speeds and through different sources.

Absence of confirmation is not proof of incompatibility. A health tool that converts missing data into alarming conclusions would quickly lose user trust.

## Decision

Health is represented by a small explicit state model:

- Compatible
- Update available
- Compatibility unknown
- Potential conflict
- Broken
- Abandoned

Every compatibility statement that changes a health state must have:

- explicit game-patch or version-range scope;
- source/provenance;
- retrieval or observation timestamp;
- evidence type/trust level.

`Unknown` remains a first-class state and is never silently mapped to `Broken`.

Resource overlap is a **Potential conflict** unless stronger evidence proves an incompatibility.

Probabilistic mod identification is distinct from exact identity.

## Consequences

- health logic is deterministic and explainable;
- the UI can expose why a result was produced;
- new Sims patches can invalidate prior certainty back to `Unknown`;
- recommendations can filter on evidence rather than assumptions.

## Guardrails

- no AI/LLM output may directly become authoritative compatibility state;
- creator/API facts and community reports remain distinguishable;
- conflicting evidence must follow a documented precedence rule or be surfaced as disputed;
- release freshness alone does not prove compatibility.
