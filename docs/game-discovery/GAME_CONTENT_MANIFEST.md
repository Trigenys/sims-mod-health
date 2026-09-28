# Game / DLC Manifest Contract

## Purpose

The Registry provides one normalized, provenance-bearing document that lets the desktop evaluate the installed game build and locally present Sims 4 content packs without coupling the app to a third-party updater schema.

The endpoint is:

```text
GET /v1/game-content/manifest
```

There is intentionally no public write endpoint in this issue.

## Registry model

The normalized manifest contains:

- schema version;
- manifest version;
- source identity and optional source URL;
- retrieval and optional expiry timestamps;
- optional SHA-256/signature metadata supplied by the ingestion pipeline;
- latest known game build;
- known game-build entries with optional sentinel fingerprints;
- content-pack entries with type, minimum game build and evidence provenance.

Example shape:

```json
{
  "schema_version": 1,
  "manifest_version": "example-1",
  "source_identity": "curated-source",
  "source_url": "https://example.invalid/evidence",
  "retrieved_at": "2026-09-28T08:00:00Z",
  "expires_at": null,
  "checksum_sha256": null,
  "signature": null,
  "latest_game_build": "1.128.90.1030",
  "game_builds": [
    {
      "version": "1.128.90.1030",
      "released_at": null,
      "fingerprints": [
        {
          "relative_path": "Game/Bin/Default.ini",
          "sha256": "..."
        }
      ]
    }
  ],
  "packs": [
    {
      "code": "EP01",
      "pack_kind": "expansion",
      "min_game_version": "1.0.0.0",
      "released_at": null,
      "expected_fingerprints": [],
      "evidence_source": "official-release-metadata",
      "evidence_url": "https://example.invalid/ep01"
    }
  ]
}
```

The example values are fixture data, not production claims about the current game release.

## Anti-corruption boundary

The desktop never consumes the Registry response directly as UI/domain state.

```text
Registry JSON
  -> RegistryGameContentManifest (wire contract)
  -> adapt_registry_manifest()
  -> GameContentManifest (desktop domain)
  -> compatibility resolver
  -> typed GameContentHealthFinding
```

This allows Registry storage or source adapters to evolve without leaking external schemas into the desktop health model.

## Game-build resolution

The resolver uses evidence in this order:

1. local `Default.ini` version captured during #73;
2. exact SHA-256 sentinel matches against known manifest builds;
3. otherwise Unknown.

A sentinel match can resolve a missing local version only when it identifies exactly one manifest build.

If local version evidence and sentinel evidence point to different builds, the result is:

```text
state = unknown
disputed = true
```

No source winner is guessed.

## Pack compatibility

For every locally present pack:

1. incomplete/unreadable local files -> `local_integrity_uncertain`;
2. no manifest metadata -> `unknown`;
3. conflicting pack metadata -> `unknown + disputed`;
4. installed game below `min_game_version` -> `game_update_required`;
5. otherwise -> `current`;
6. when only cached Registry metadata is available, a non-blocking result is visibly marked stale.

Installed pack presence is not ownership/entitlement evidence.

## Offline cache

The latest successfully fetched manifest is stored locally in SQLite table:

```text
game_content_manifest_cache
```

When the Registry cannot be reached:

- the cache is still evaluated;
- `manifest_state` becomes `cached_stale`;
- findings retain `manifest_stale = true`;
- the UI can therefore distinguish current evidence from offline cached evidence.

If neither Registry nor cache is available, Game/DLC update conclusions remain Unknown.

## Health states

Typed Game/DLC findings use:

- `current`
- `update_available`
- `game_update_required`
- `metadata_stale`
- `local_integrity_uncertain`
- `unknown`

There is deliberately no `broken` state derived solely from absent metadata.

## Security and source boundaries

This manifest contains compatibility and integrity metadata only.

It must not contain or link to:

- entitlement bypass instructions;
- DLC unlocker metadata;
- torrent indexes;
- unlicensed game/DLC payload mirrors;
- ownership claims inferred from local files.

Direct patch payloads remain outside #74 and behind the separate #77 gate.
