# Artifact-to-mod matching engine

## Purpose

The registry resolves a scanned local artifact to a canonical registry artifact, release and mod while preserving the evidence level used to make the decision.

The engine never upgrades a probabilistic guess into an exact identity.

## Resolution evidence

The engine accepts a bounded batch of local probes containing fingerprints plus optional technical metadata:

- SHA-256 and CurseForge source fingerprints;
- DBPF resource signatures;
- TS4Script identity signatures;
- normalized filename and file size;
- optional embedded creator, mod-name and version hints.

Raw mod bytes and absolute local paths remain outside the registry boundary.

## Matching cascade

Evidence is evaluated in the product priority order:

1. exact SHA/source fingerprint;
2. embedded creator/mod/version hints;
3. normalized filename and creator aliases;
4. structural resource/script signature;
5. fuzzy name similarity.

Exact fingerprint matches short-circuit probabilistic matching.

For non-exact resolution, evidence can reinforce a candidate. Structural signatures and metadata are never exposed with `confidence: exact`.

## Resolution states

Each probe returns one of:

- `resolved` — one candidate clears the selection threshold and is separated from the runner-up;
- `ambiguous` — multiple candidates are too close to choose safely;
- `unresolved` — no candidate has enough evidence.

`selected_artifact_id` is populated only for `resolved`.

Ambiguous and unresolved probes may still return bounded candidate explanations for the UI, but the engine does not silently pick one.

## Confidence

- `exact` is reserved for deterministic SHA/source fingerprint matches;
- `high`, `medium` and `low` are probabilistic confidence bands;
- every candidate includes a numeric score and structured evidence.

Candidate ordering is deterministic: score descending, then artifact UUID.

## Batch behavior

Exact and structural fingerprint lookups are performed in batch. Candidate catalogs are loaded once per artifact kind for the request and reused across probes.

The current fuzzy candidate pool is intentionally bounded to 1,000 registry artifacts per artifact kind. This keeps the first implementation predictable. If registry scale makes this bound material, the next step is a PostgreSQL-native candidate index (for example trigram/search-vector preselection), not an unbounded in-memory scan.

## Safety

Matching is identity inference only. It does not create compatibility reports, download files, or mutate local installations.
