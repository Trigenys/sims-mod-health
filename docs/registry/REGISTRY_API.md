# Registry API

## Purpose

The Registry API is the shared source of truth for public mod identity and health evidence.

It complements the offline-first desktop app:

- **desktop/local:** files, folders, raw package/script contents, scan cache, diagnostics;
- **registry/shared:** creators, mods, releases, public artifacts, fingerprints, patches and sourced compatibility statements.

Raw local mod files are not required by the registry contract.

## Stack

The service is a modular monolith under:

```text
services/registry-api/
├── app/
│   ├── api/                 HTTP/versioning/error boundary
│   ├── db/                  SQLAlchemy engine/session
│   ├── domain/              registry entities
│   ├── repositories/        persistence/query layer
│   ├── schemas/             Pydantic contracts
│   └── services/            application orchestration
├── migrations/              Alembic
└── tests/
```

Target runtime:

- FastAPI
- PostgreSQL
- SQLAlchemy 2
- Alembic
- Pydantic 2

Redis/workers are intentionally absent until ingestion volume proves a need.

## Core schema

Version 1 creates:

- Creator
- Mod
- ModRelease
- Artifact
- Fingerprint
- Source
- GamePatch
- CompatibilityReport
- DependencyRule
- ConflictRule

### Provenance

Health-affecting shared records must preserve evidence origin.

`GamePatch` and `CompatibilityReport` require:

- `source_id`
- `retrieved_at`

They may also retain:

- source URL
- source record identifier
- notes

A compatibility status without source provenance is not valid shared registry evidence.

## Batch resolution contract

Endpoint:

```text
POST /v1/artifacts/resolve
```

A desktop probe may include only bounded technical metadata:

```json
{
  "artifacts": [
    {
      "client_ref": "local-42",
      "artifact_kind": "package",
      "filename": "renamed.package",
      "size_bytes": 12345,
      "identity_hints": {
        "creator": "Creator",
        "mod_name": "Example Mod",
        "version": "2.4.1"
      },
      "fingerprints": [
        {
          "kind": "sha256",
          "value": "<hex>",
          "algorithm_version": "sha256-v1"
        }
      ]
    }
  ]
}
```

The contract does **not** accept:

- raw `.package` bytes;
- raw `.ts4script` bytes;
- base64 file bodies;
- Sims saves/households/screenshots;
- absolute local filesystem paths.

Unknown fields are rejected by Pydantic with `extra="forbid"`.

## Resolution semantics

The resolver now implements the #13 evidence cascade.

Deterministic evidence:

- SHA-256;
- CurseForge source fingerprint.

Only deterministic fingerprint matches may return `confidence: "exact"`.

Probabilistic evidence may include:

- embedded creator/mod/version hints;
- normalized filename and creator aliases;
- DBPF resource signatures;
- TS4Script identity signatures;
- fuzzy filename/mod-name similarity.

Every response reports a resolution state:

- `resolved` when one candidate is safely selected;
- `ambiguous` when candidates are too close to choose;
- `unresolved` when evidence is insufficient.

Probabilistic candidates retain structured evidence, confidence and score. Ambiguous candidates are never silently selected.

See `docs/registry/MATCHING_ENGINE.md` for scoring, batching and candidate-bound details.

## Health evaluation contract

Endpoint:

```text
POST /v1/health/evaluate
```

The desktop sends resolved registry release IDs and the current normalized Sims patch:

```json
{
  "patch_version": "1.128.90.1030",
  "platform": "windows",
  "installed_release_ids": ["<uuid>"]
}
```

Each item returns the final triage state separately from the underlying compatibility state. Update availability is computed independently and includes the target release compatibility state, so a newer version is never mistaken for a confirmed-compatible version.

Compatibility evidence is patch-scoped, source-provenanced and inspectable. Exact-patch evidence outranks range evidence. Conflicting current evidence is returned as `unknown` with `disputed: true` instead of silently picking one source.

See `docs/health/PATCH_COMPATIBILITY_ENGINE.md`.

## Discovery recommendation contract

Endpoint:

```text
POST /v1/discovery/recommend
```

The request contains the current patch plus resolved installed release IDs.

Discovery uses deterministic canonical categories/features, requires explicit compatible current-patch evidence, excludes already-installed mods, abandoned/broken/potential-conflict releases and known incompatibilities, then returns deterministic “Because you use…” explanations.

There is no sponsored-placement field or ranking input in the MVP.

See `docs/discovery/RECOMMENDATION_ENGINE.md`.

## Installation relationship contract

Endpoint:

```text
POST /v1/health/relationships/evaluate
```

The request contains only resolved installed release IDs. The registry evaluates required dependencies and known incompatibility rules without receiving local file paths or DBPF resource keys.

The response includes actionable missing/outdated dependency findings, version-constrained incompatibilities with provenance, reverse dependency usage (`used_by_count`) and detected dependency cycles.

Exact duplicate hashes and DBPF resource overlap remain local desktop analysis. See `docs/health/DEPENDENCY_CONFLICT_ANALYSIS.md`.

## Validation errors

Pydantic validation errors are normalized to:

```json
{
  "error": {
    "code": "validation_error",
    "message": "Request validation failed.",
    "details": [
      {
        "field": "artifacts.0.fingerprints",
        "message": "...",
        "type": "..."
      }
    ]
  }
}
```

This keeps client handling stable instead of leaking framework-specific response shape changes.

## Migration verification

CI launches a real PostgreSQL service and runs:

```text
alembic upgrade head
alembic downgrade base
alembic upgrade head
pytest -q
```

This proves the versioned schema can create a fresh database deterministically and survive a clean rollback/reapply cycle.

## Local development

From `services/registry-api`:

```bash
pip install -e ".[dev]"
export REGISTRY_DATABASE_URL="postgresql+psycopg://..."
alembic upgrade head
uvicorn app.main:app --reload
```
