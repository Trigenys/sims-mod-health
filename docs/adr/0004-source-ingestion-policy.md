# ADR-0004 — Source-respectful registry ingestion

**Status:** Accepted  
**Decision date:** 2026-09-25  
**Issue:** #2

## Context

The registry needs metadata from multiple mod ecosystems. A generalized scraper would be brittle and can violate source terms, creator expectations or redistribution rules.

## Decision

The registry uses explicit source adapters with this preference order:

1. official structured API;
2. creator-published machine-readable manifest;
3. public GitHub release metadata;
4. creator page that explicitly permits automated access;
5. moderated community report with provenance.

The product does not:

- bypass authentication or paywalls;
- bypass rate limits;
- scrape a source that prohibits automated access;
- redistribute mod files without permission;
- mirror creator-gated or paid content;
- remove creator attribution.

A source outage produces a stale/unknown state, not an invented compatibility result.

## Consequences

- adapters have clearer contracts and failure modes;
- source provenance becomes mandatory registry data;
- catalog breadth may grow more slowly than with unrestricted scraping;
- creator trust and long-term maintainability take precedence over short-term coverage.

## Guardrails

Every connector issue must review:

- source terms/API contract;
- rate limits;
- deletion/withdrawal behavior;
- attribution;
- provenance storage;
- stale-source behavior.
