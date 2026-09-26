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
