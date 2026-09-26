# Diagnostic parsing and implicated-mod resolution

## Purpose

Diagnostics are treated as **correlation evidence**, not proof of causality.

The engine normalizes supported Sims reports, maps module/file/resource references to installed local artifacts, then asks the Registry matching engine for canonical identity when available.

The UI deliberately uses language such as **Implicated candidate** and **Correlated with report evidence**. It does not convert a traceback reference into a causal claim.

## Supported report families

The local parser currently recognizes recent files whose names identify:

- `lastException`;
- `lastUIException`;
- MCCC `mc_lastexception` / MCCC reports;
- Better Exceptions / exception-report HTML.

The parser checks the current Sims user root plus bounded MCCC / Better Exceptions report directories. It does not recursively crawl the entire user profile.

Each report is limited to 4 MiB and the newest 20 supported reports are considered.

## Normalized observations

Supported evidence is normalized into three observation types.

### File

A report explicitly names an installed `.package` or `.ts4script` filename.

This is the strongest local diagnostic link and receives exact local-match confidence when the basename maps to one enabled local artifact.

### Python module

Traceback/module references are compared against module and package names exposed by the read-only TS4Script inspector.

The inspector never imports or executes bundled Python.

Exact module ownership is high-confidence correlation evidence. Package-namespace ownership is slightly weaker.

Known Sims/game framework module roots are ignored so the engine does not implicate mods merely because a traceback passes through core game code.

### DBPF resource

Resource references in the form:

```text
0xTYPE:0xGROUP:0xINSTANCE
```

are compared against live resource keys from enabled DBPF packages.

A resource-key match identifies an installed package containing that resource. It remains correlation evidence rather than a statement that the package is broken.

## Candidate confidence

Local confidence is deterministic from evidence type:

- direct installed filename: exact;
- exact TS4Script module: high;
- TS4Script package namespace: high;
- DBPF resource ownership: high.

Multiple independent evidence items can raise the numeric score, but never above 99 because diagnostic evidence alone is not a causal proof.

Every candidate exposes the evidence records that produced the score.

## Canonical identity

Only implicated local artifacts are sent to `/v1/artifacts/resolve`.

The request reuses existing safe artifact evidence:

- basename;
- artifact kind;
- file size;
- creator/version hints when available;
- supported fingerprints.

The local relative path is not sent.

When the Registry returns a resolved selected artifact, the diagnostic candidate receives canonical mod/release IDs and the Registry's own matching confidence/deterministic flag.

If Registry access is offline or partial, local diagnostic evidence remains usable.

## Persistence

The existing local `diagnostics` table stores:

- installation ID;
- source kind;
- report filename only;
- content SHA-256;
- parse status;
- normalized summary JSON;
- observed/parsed timestamps.

Raw report content and absolute report paths are not persisted.

Identical report content/source/file combinations are updated rather than duplicated.

## Malformed and unsupported reports

Failure is per report.

Examples:

- empty file → `malformed`;
- over 4 MiB → `unsupported`;
- read failure → `error`;
- valid supported report with no recognized mod references → `parsed` with zero candidates.

One malformed report does not block other reports.

## Privacy and telemetry preview

Before text is normalized, the parser redacts:

- the Sims user-root absolute path;
- Windows `C:\Users\<name>` paths;
- macOS `/Users/<name>` paths;
- Linux `/home/<name>` paths;
- email addresses;
- account/user/player/persona IDs.

The optional telemetry preview contains only:

- source kind;
- observation kinds;
- candidate count;
- number of redactions applied.

It does not contain raw report text, absolute paths, email addresses or player IDs.

## Product surface

The Diagnostics page displays:

- source/report parse state;
- implicated candidates;
- local confidence score;
- evidence type/reference/explanation;
- canonical resolution state;
- privacy-redaction count;
- normalized observations behind a disclosure control.

The wording intentionally avoids assigning blame from correlation evidence.
