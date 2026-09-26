# CurseForge source adapter

## Contract

Sims Mod Health uses the official CurseForge API only.

- Base URL: `https://api.curseforge.com`
- Authentication: `x-api-key`
- File pagination: maximum 50 rows per page
- CurseForge result window: maximum 10,000 rows

No CurseForge website pages are scraped.

The API key is supplied at runtime through `CURSEFORGE_API_KEY`; it is never committed to the repository.

## Internal architecture

```text
sources/curseforge/
├── client.py       # HTTP gateway, retry/backoff, pagination
├── models.py       # source response contracts
└── mapper.py       # anti-corruption layer into registry concepts

repositories/
└── curseforge_ingestion.py

services/
└── curseforge_sync.py
```

Patterns:

- **Gateway / Adapter**: `CurseForgeClient` owns external HTTP behavior.
- **Anti-Corruption Layer**: `CurseForgeMapper` prevents source-specific payload shape from leaking into the domain.
- **Repository**: source-aware upserts and fingerprint replacement stay out of HTTP code.
- **Application Service**: `CurseForgeSyncService` coordinates fetch → map → persist and turns connector outages into explicit sync states.

## Retry and availability

Retryable conditions:

- HTTP 408
- HTTP 429
- HTTP 500
- HTTP 502
- HTTP 503
- HTTP 504
- transport/network failures

The adapter uses bounded exponential backoff.

When `Retry-After` is supplied, its value is retained as source guidance but the local blocking sleep is capped by `max_backoff_seconds`. This prevents one upstream response from tying up a worker indefinitely.

After retry exhaustion, the adapter returns an explicit `unavailable` sync result. Existing registry data remains untouched.

404 is represented separately as `not_found`.

## Ingested project data

Project mapping preserves:

- CurseForge project id
- game id
- project name / slug / summary
- project URL
- author ids, names and URLs
- availability/status
- source retrieval timestamp

The first CurseForge author is used as the current canonical registry creator for this initial adapter. Other authors remain preserved as aliases/source metadata so the original multi-author evidence is not lost.

## Ingested file/release data

Each CurseForge file maps to one source-aware ModRelease and Artifact keyed by:

```text
(source_id, source_record_id=file_id)
```

This makes ingestion idempotent.

Stored evidence includes:

- file id
- display name
- filename
- release type
- file status
- file date
- file length
- download URL
- game-version strings
- dependency relationships
- module metadata
- CurseForge file fingerprint
- SHA-1 / MD5 when provided
- optional changelog

Changelogs are not fetched by default because doing so would add one HTTP request per file. They can be selectively enabled during a sync.

## Fingerprints

Registry fingerprint kinds added by this adapter:

- `curseforge` — CurseForge file fingerprint
- `sha1` — source-provided SHA-1
- `md5` — source-provided MD5

The public desktop resolution contract still treats only the existing deterministic exact kinds from #10 as exact input. Adding source hashes to registry storage does not silently change client matching semantics.

## Dependencies and game versions

CurseForge dependency relations and game-version strings are retained in the artifact's source metadata.

They are **evidence**, not health conclusions.

The adapter does not create `CompatibilityReport` records from those fields. Compatibility interpretation remains the responsibility of the later compatibility engine, and normalized dependency/conflict analysis remains downstream work.

## Failure isolation

CurseForge sync is not executed inside the normal artifact-resolution request path.

If CurseForge is unavailable:

- no fabricated compatibility status is written;
- no partial ingestion is committed;
- existing registry rows remain usable;
- `POST /v1/artifacts/resolve` continues operating from persisted data.

## Testing

The CI suite uses mocked HTTP transports, never a real API key.

Tests cover:

- API-key header
- official endpoint paths
- pagination
- 429 + `Retry-After`
- bounded retry sleeps
- 503 retry exhaustion
- representative project/file mapping
- hashes/fingerprints
- game versions/dependencies
- idempotent persistence
- optional changelog
- source-unavailable isolation
- zero fabricated compatibility reports
