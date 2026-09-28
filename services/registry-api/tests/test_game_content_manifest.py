from __future__ import annotations

from datetime import datetime, timedelta, timezone

from fastapi.testclient import TestClient

from app.domain.game_content import GameContentManifestDocument
from app.main import app


client = TestClient(app)


def _document() -> GameContentManifestDocument:
    retrieved = datetime(2026, 9, 28, 8, 0, tzinfo=timezone.utc)
    return GameContentManifestDocument(
        schema_version=1,
        manifest_version="2026-09-28.1",
        source_identity="trigenys-curated",
        source_url="https://registry.example.invalid/evidence",
        retrieved_at=retrieved,
        expires_at=retrieved + timedelta(days=7),
        checksum_sha256="a" * 64,
        signature=None,
        payload_json={
            "latest_game_build": "1.128.90.1030",
            "game_builds": [
                {
                    "version": "1.128.90.1030",
                    "released_at": "2026-09-20T00:00:00Z",
                    "fingerprints": [
                        {
                            "relative_path": "Game/Bin/Default.ini",
                            "sha256": "b" * 64,
                        }
                    ],
                }
            ],
            "packs": [
                {
                    "code": "EP01",
                    "pack_kind": "expansion",
                    "min_game_version": "1.0.0.0",
                    "released_at": "2015-03-31T00:00:00Z",
                    "expected_fingerprints": [],
                    "evidence_source": "official-release-metadata",
                    "evidence_url": "https://example.invalid/ep01",
                }
            ],
        },
    )


def test_manifest_endpoint_returns_provenance_and_payload(db_session) -> None:
    db_session.add(_document())
    db_session.commit()

    response = client.get("/v1/game-content/manifest")

    assert response.status_code == 200
    payload = response.json()
    assert payload["manifest_version"] == "2026-09-28.1"
    assert payload["source_identity"] == "trigenys-curated"
    assert payload["latest_game_build"] == "1.128.90.1030"
    assert payload["packs"][0]["code"] == "EP01"


def test_manifest_endpoint_is_404_when_registry_has_no_document() -> None:
    response = client.get("/v1/game-content/manifest")

    assert response.status_code == 404
    assert response.json()["detail"]["code"] == "game_content_manifest_unavailable"
