# Security hardening review

Review date: 2026-09-27  
Release gate: issue #21

## Decision

The beta security gate is considered satisfied only when all of the following are true on the release candidate commit:

- Rust CI is green;
- Registry CI is green;
- Frontend CI is green;
- Security CI is green;
- there are no separately tracked open critical/high security defects;
- the threat matrix below has no unreviewed applicable item.

Issue #21 is the umbrella hardening task and is not itself counted as an unresolved defect after its acceptance evidence is complete.

## Threat evidence matrix

| Threat | Status | Evidence / mitigation | Residual risk |
| --- | --- | --- | --- |
| Malformed DBPF | Verified | bounded header/index parsing, checked arithmetic, resource/index limits, adversarial mutation harness | malformed-but-valid edge cases may still be rejected rather than parsed |
| TS4Script execution | Verified | archive inspected as data only; no Python import/execution; read-only source tests | none accepted beyond parser bugs covered by normal defect process |
| Archive bombs | Verified | archive byte/entry/expanded-size/ratio/path-depth/metadata budgets; Security CI archive-bomb tests | compressed formats supported by the ZIP crate remain dependency trust |
| Archive/path traversal | Verified | unsafe ZIP paths rejected; update targets require normal relative components; symlink parents rejected | none accepted |
| Untrusted downloads | Verified | HTTPS adapter allowlists, redirect revalidation, size cap, staged install, SHA-256 evidence, restore point before mutation | expected upstream hash may be unavailable; observed hash is still recorded and rechecked |
| Rollback abuse | Verified | restore copy hash checked; rollback refuses to overwrite an independently changed target | user may need manual recovery if they deliberately alter files during an interrupted transaction |
| Registry poisoning | Mitigated / accepted residual | per-statement provenance, patch scoping, conflicting-source dispute handling, unknown-first behavior | a trusted upstream can publish incorrect facts; UI exposes provenance instead of silently treating source claims as ground truth |
| Source outage / stale evidence | Verified | absence of current evidence returns Unknown; offline Overview keeps local data and never fabricates Broken | availability loss can reduce certainty |
| False-positive conflict claims | Verified | resource overlap remains Potential conflict; stronger Broken states require explicit sourced evidence | users must still interpret potential conflicts |
| Privacy leakage | Verified | local raw files stay local; diagnostic redaction; raw diagnostic bodies/absolute paths are not persisted; telemetry consent defaults off | relative filenames can still be diagnostic evidence locally |
| Diagnostic telemetry consent | Verified | consent preference defaults false, malformed preference fails closed, user can opt in/revoke; no automatic raw-report upload exists | future telemetry transport must consume only the redacted preview and preserve this gate |
| Excessive Tauri permissions | Verified | main capability contains only `core:default`; Security CI rejects capability drift and broad fs/shell/http/process permissions | no broad WebView filesystem capability accepted |
| WebView content injection | Verified with accepted low residual | explicit production CSP: self-only scripts, no unsafe-eval, no objects, no framing, no external network from WebView | `style-src 'unsafe-inline'` is temporarily accepted because product UI uses inline style values for progress/score rendering; it does not permit script execution |
| Dependency vulnerabilities | Release-gated | Security CI audits JS, Python and Rust dependency graphs and fails on known high/critical findings according to each ecosystem audit tool | zero-day/unpublished advisories cannot be detected |
| Secrets / signed URL leakage | Verified | signed URL query/fragment stripped before update provenance is journaled; no secret is required in frontend configuration | upstream HTTP error bodies must remain treated as untrusted text |
| Update dependency regression | Verified | complete installed relationship graph checked before archive/remove; unresolved graph blocks mutation | safe update coverage is intentionally reduced when identity is incomplete |

## WebView capability audit

The only declared main-window capability is:

```json
["core:default"]
```

Filesystem, shell, process and arbitrary HTTP plugins are not granted to the WebView.

Privileged operations stay in explicit Rust commands:

- scan/discovery of Sims installation paths;
- local parser access;
- update/rollback mutation;
- privacy preference persistence.

`scripts/security/audit-tauri-config.mjs` is the executable guardrail for this boundary.

## Content Security Policy

Production CSP is explicit.

The WebView may load application code/assets from itself and communicate through Tauri IPC. It cannot evaluate strings as scripts, embed arbitrary objects, be framed, or make arbitrary web requests directly.

The native Rust registry client remains the network boundary.

Accepted residual risk: inline styles are allowed because current React surfaces use runtime style attributes for progress widths and score variables. Removing that exception is a future hardening improvement, not a prerequisite for beta because scripts remain self-only and `unsafe-eval` is forbidden.

## Parser/adversarial verification

Security CI runs deterministic adversarial suites on Windows for:

- DBPF mutation/no-panic behavior;
- TS4Script mutation/no-panic behavior;
- ZIP bomb limits;
- archive/update path traversal;
- update source allowlists;
- rollback refusal when target state changed independently.

The ordinary Rust CI still runs the entire native test suite.

These deterministic mutation suites are not a substitute for continuous coverage-guided fuzzing. For the first beta, the accepted approach is repeatable adversarial CI plus strict resource limits. Coverage-guided fuzzing can be added later without weakening the beta gate.

## Dependency audit

Security CI builds transient lock graphs and audits:

- npm dependency graph with `npm audit --audit-level=high`;
- Registry Python requirements with `pip-audit`;
- Rust dependency graph with `cargo-audit`.

A failing known-vulnerability audit blocks the security gate.

## Privacy defaults

Diagnostic telemetry consent is stored locally in SQLite under:

```text
privacy.diagnostic_telemetry_enabled
```

Default: `false`.

Invalid stored values fail closed to `false`.

The current product has no automatic raw diagnostic upload. Consent only permits future redacted diagnostic telemetry; it does not authorize raw reports, absolute paths, email addresses, account/player IDs, saves or screenshots.

## Critical/high defect gate

At the time this hardening review was prepared, the repository had no separately tracked open critical/high defect issue besides the umbrella #21 hardening task.

This statement is not permanent evidence by itself. The beta release commit must also have a green Security CI dependency audit and a fresh open-issue review before #22 is completed.
