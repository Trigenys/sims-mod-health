# ADR-0002 — Rust native privilege boundary

**Status:** Accepted  
**Decision date:** 2026-09-25  
**Issue:** #2

## Context

The React WebView needs to present local scan results but should not have unrestricted access to the user's filesystem.

The product will parse untrusted files and archives. A compromised frontend or overly broad Tauri capability must not automatically become unrestricted filesystem access.

## Decision

Privileged operations remain behind **narrow Rust commands** in the Tauri native layer.

The React/TypeScript frontend owns:

- rendering;
- interaction;
- view state;
- presentation-level validation.

The Rust layer owns:

- filesystem discovery and traversal;
- game-version reads;
- DBPF parsing;
- TS4Script archive inspection;
- hashing/fingerprinting;
- SQLite access;
- diagnostics parsing;
- backup, staging and rollback;
- any future file mutation.

The default WebView capability set remains minimal. New Tauri permissions require an explicit security review.

## Consequences

- scanner/parsers can use checked Rust APIs and bounded reads;
- privileged operations are independently testable;
- frontend compromise has a smaller blast radius;
- additional Rust implementation work is accepted in exchange for a clearer trust boundary.

## Guardrails

- no direct broad filesystem plugin permission for the WebView;
- script-mod contents are inspected as data and never executed;
- archive extraction is not required for inspection and must not write arbitrary paths;
- every new privileged command validates inputs at the native boundary.
