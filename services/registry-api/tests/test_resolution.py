from __future__ import annotations

from datetime import datetime, timezone

from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from app.domain import Artifact, Creator, Fingerprint, Mod, ModRelease, Source
from app.main import app


client = TestClient(app)


def seed_artifact(session: Session) -> Artifact:
    now = datetime.now(timezone.utc)
    source = Source(
        kind="github",
        external_id="creator/example-mod",
        name="Example Mod GitHub",
        base_url="https://github.com/creator/example-mod",
        metadata_json={},
        created_at=now,
    )
    creator = Creator(
        slug="creator",
        display_name="Creator",
        aliases=[],
        created_at=now,
        updated_at=now,
    )
    session.add_all([source, creator])
    session.flush()

    mod = Mod(
        creator_id=creator.id,
        slug="example-mod",
        name="Example Mod",
        aliases=[],
        created_at=now,
    )
    session.add(mod)
    session.flush()

    release = ModRelease(
        mod_id=mod.id,
        version="2.4.1",
        source_id=source.id,
        source_url="https://github.com/creator/example-mod/releases/tag/v2.4.1",
        retrieved_at=now,
    )
    session.add(release)
    session.flush()

    artifact = Artifact(
        release_id=release.id,
        artifact_kind="package",
        filename="example.package",
        size_bytes=12345,
        source_id=source.id,
        source_url="https://example.invalid/example.package",
        retrieved_at=now,
        metadata_json={},
    )
    session.add(artifact)
    session.flush()

    session.add_all(
        [
            Fingerprint(
                artifact_id=artifact.id,
                kind="sha256",
                value="a" * 64,
                algorithm_version="sha256-v1",
                created_at=now,
            ),
            Fingerprint(
                artifact_id=artifact.id,
                kind="resource_signature",
                value="resource-signature",
                algorithm_version="dbpf-resource-keys-v1",
                created_at=now,
            ),
        ]
    )
    session.commit()
    return artifact


def test_batch_resolution_uses_exact_fingerprint_evidence_only(
    db_session: Session,
) -> None:
    artifact = seed_artifact(db_session)

    response = client.post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "exact",
                    "artifact_kind": "package",
                    "filename": "renamed.package",
                    "fingerprints": [
                        {
                            "kind": "sha256",
                            "value": "a" * 64,
                            "algorithm_version": "sha256-v1",
                        },
                        {
                            "kind": "resource_signature",
                            "value": "resource-signature",
                            "algorithm_version": "dbpf-resource-keys-v1",
                        },
                    ],
                },
                {
                    "client_ref": "structural-only",
                    "fingerprints": [
                        {
                            "kind": "resource_signature",
                            "value": "resource-signature",
                            "algorithm_version": "dbpf-resource-keys-v1",
                        }
                    ],
                },
            ]
        },
    )

    assert response.status_code == 200
    payload = response.json()
    assert [item["client_ref"] for item in payload["artifacts"]] == [
        "exact",
        "structural-only",
    ]

    exact = payload["artifacts"][0]["matches"]
    assert len(exact) == 1
    assert exact[0]["artifact_id"] == str(artifact.id)
    assert exact[0]["confidence"] == "exact"
    assert exact[0]["evidence"] == [
        {
            "type": "exact_fingerprint",
            "kind": "sha256",
            "value": "a" * 64,
            "algorithm_version": "sha256-v1",
        }
    ]

    assert payload["artifacts"][1]["matches"] == []


def test_unknown_exact_fingerprint_returns_empty_match_set() -> None:
    response = client.post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "unknown",
                    "fingerprints": [
                        {
                            "kind": "sha256",
                            "value": "f" * 64,
                            "algorithm_version": "sha256-v1",
                        }
                    ],
                }
            ]
        },
    )

    assert response.status_code == 200
    assert response.json() == {
        "artifacts": [{"client_ref": "unknown", "matches": []}]
    }
