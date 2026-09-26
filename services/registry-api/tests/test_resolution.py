from __future__ import annotations

from datetime import datetime, timezone

from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from app.domain import Artifact, Creator, Fingerprint, Mod, ModRelease, Source
from app.main import app


client = TestClient(app)


def seed_artifact(
    session: Session,
    *,
    source_external_id: str,
    creator_slug: str,
    creator_name: str,
    creator_aliases: list[str] | None = None,
    mod_slug: str,
    mod_name: str,
    mod_aliases: list[str] | None = None,
    version: str,
    filename: str,
    size_bytes: int,
    fingerprints: list[tuple[str, str, str]] | None = None,
) -> Artifact:
    now = datetime.now(timezone.utc)
    source = Source(
        kind="test",
        external_id=source_external_id,
        name=mod_name,
        base_url="https://example.invalid",
        metadata_json={},
        created_at=now,
    )
    creator = Creator(
        slug=creator_slug,
        display_name=creator_name,
        aliases=creator_aliases or [],
        created_at=now,
        updated_at=now,
    )
    session.add_all([source, creator])
    session.flush()

    mod = Mod(
        creator_id=creator.id,
        slug=mod_slug,
        name=mod_name,
        aliases=mod_aliases or [],
        created_at=now,
    )
    session.add(mod)
    session.flush()

    release = ModRelease(
        mod_id=mod.id,
        version=version,
        source_id=source.id,
        source_record_id=f"release-{source_external_id}",
        source_url="https://example.invalid/release",
        retrieved_at=now,
    )
    session.add(release)
    session.flush()

    artifact = Artifact(
        release_id=release.id,
        artifact_kind="package",
        filename=filename,
        size_bytes=size_bytes,
        source_id=source.id,
        source_record_id=f"artifact-{source_external_id}",
        source_url="https://example.invalid/artifact",
        retrieved_at=now,
        metadata_json={},
    )
    session.add(artifact)
    session.flush()

    for kind, value, algorithm_version in fingerprints or []:
        session.add(
            Fingerprint(
                artifact_id=artifact.id,
                kind=kind,
                value=value,
                algorithm_version=algorithm_version,
                created_at=now,
            )
        )

    session.commit()
    return artifact


def test_exact_fingerprint_is_deterministic_and_selected(
    db_session: Session,
) -> None:
    artifact = seed_artifact(
        db_session,
        source_external_id="exact",
        creator_slug="creator",
        creator_name="Creator",
        mod_slug="example-mod",
        mod_name="Example Mod",
        version="2.4.1",
        filename="example.package",
        size_bytes=12345,
        fingerprints=[
            ("sha256", "a" * 64, "sha256-v1"),
            (
                "resource_signature",
                "resource-signature",
                "dbpf-resource-keys-v1",
            ),
        ],
    )

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
                }
            ]
        },
    )

    assert response.status_code == 200
    resolution = response.json()["artifacts"][0]
    assert resolution["status"] == "resolved"
    assert resolution["selected_artifact_id"] == str(artifact.id)
    assert len(resolution["matches"]) == 1
    match = resolution["matches"][0]
    assert match["confidence"] == "exact"
    assert match["score"] == 1.0
    assert match["deterministic"] is True
    assert match["stage"] == "exact_fingerprint"
    assert match["evidence"] == [
        {
            "type": "exact_fingerprint",
            "kind": "sha256",
            "value": "a" * 64,
            "algorithm_version": "sha256-v1",
        }
    ]


def test_embedded_metadata_resolves_without_claiming_exact(
    db_session: Session,
) -> None:
    artifact = seed_artifact(
        db_session,
        source_external_id="metadata",
        creator_slug="deaderpool",
        creator_name="Deaderpool",
        creator_aliases=["DP"],
        mod_slug="mc-command-center",
        mod_name="MC Command Center",
        mod_aliases=["MCCC"],
        version="2026.5.0",
        filename="mc_cmd_center.package",
        size_bytes=500,
    )

    response = client.post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "metadata",
                    "artifact_kind": "package",
                    "identity_hints": {
                        "creator": "DP",
                        "mod_name": "MCCC",
                        "version": "2026.5.0",
                    },
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
    resolution = response.json()["artifacts"][0]
    assert resolution["status"] == "resolved"
    assert resolution["selected_artifact_id"] == str(artifact.id)
    match = resolution["matches"][0]
    assert match["stage"] == "embedded_metadata"
    assert match["confidence"] == "high"
    assert match["deterministic"] is False
    assert {item["field"] for item in match["evidence"] if item["type"] == "embedded_metadata"} == {
        "creator",
        "mod_name",
        "version",
    }


def test_structural_signature_can_resolve_but_is_not_exact(
    db_session: Session,
) -> None:
    artifact = seed_artifact(
        db_session,
        source_external_id="structural",
        creator_slug="creator-structural",
        creator_name="Structural Creator",
        mod_slug="structural-mod",
        mod_name="Structural Mod",
        version="1.0",
        filename="structural.package",
        size_bytes=700,
        fingerprints=[
            (
                "resource_signature",
                "resource-123",
                "dbpf-resource-keys-v1",
            )
        ],
    )

    response = client.post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "structural",
                    "artifact_kind": "package",
                    "fingerprints": [
                        {
                            "kind": "resource_signature",
                            "value": "resource-123",
                            "algorithm_version": "dbpf-resource-keys-v1",
                        }
                    ],
                }
            ]
        },
    )

    assert response.status_code == 200
    resolution = response.json()["artifacts"][0]
    assert resolution["status"] == "resolved"
    assert resolution["selected_artifact_id"] == str(artifact.id)
    match = resolution["matches"][0]
    assert match["stage"] == "resource_signature"
    assert match["confidence"] == "medium"
    assert match["deterministic"] is False


def test_fuzzy_filename_returns_probabilistic_candidate(
    db_session: Session,
) -> None:
    artifact = seed_artifact(
        db_session,
        source_external_id="fuzzy",
        creator_slug="turbodriver",
        creator_name="TURBODRIVER",
        mod_slug="wonderfulwhims",
        mod_name="WonderfulWhims",
        version="55",
        filename="WonderfulWhims_v55.package",
        size_bytes=900,
    )

    response = client.post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "fuzzy",
                    "artifact_kind": "package",
                    "filename": "WonderfulWhims.package",
                    "fingerprints": [
                        {
                            "kind": "sha256",
                            "value": "e" * 64,
                            "algorithm_version": "sha256-v1",
                        }
                    ],
                }
            ]
        },
    )

    assert response.status_code == 200
    resolution = response.json()["artifacts"][0]
    assert resolution["status"] == "resolved"
    assert resolution["selected_artifact_id"] == str(artifact.id)
    match = resolution["matches"][0]
    assert match["confidence"] != "exact"
    assert match["deterministic"] is False
    assert match["stage"] in {"normalized_name", "fuzzy_candidate"}


def test_ambiguous_candidates_remain_unselected(
    db_session: Session,
) -> None:
    seed_artifact(
        db_session,
        source_external_id="ambiguous-a",
        creator_slug="creator-a",
        creator_name="Creator A",
        mod_slug="better-build-buy-a",
        mod_name="Better Build Buy A",
        version="1",
        filename="BetterBuildBuy_A.package",
        size_bytes=100,
    )
    seed_artifact(
        db_session,
        source_external_id="ambiguous-b",
        creator_slug="creator-b",
        creator_name="Creator B",
        mod_slug="better-build-buy-b",
        mod_name="Better Build Buy B",
        version="1",
        filename="BetterBuildBuy_B.package",
        size_bytes=100,
    )

    response = client.post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "ambiguous",
                    "artifact_kind": "package",
                    "filename": "BetterBuildBuy.package",
                    "size_bytes": 100,
                    "fingerprints": [
                        {
                            "kind": "sha256",
                            "value": "d" * 64,
                            "algorithm_version": "sha256-v1",
                        }
                    ],
                }
            ]
        },
    )

    assert response.status_code == 200
    resolution = response.json()["artifacts"][0]
    assert resolution["status"] == "ambiguous"
    assert resolution["selected_artifact_id"] is None
    assert len(resolution["matches"]) == 2
    assert all(match["confidence"] != "exact" for match in resolution["matches"])


def test_unknown_artifact_is_unresolved() -> None:
    response = client.post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "unknown",
                    "artifact_kind": "package",
                    "filename": "totally-unrelated.package",
                    "fingerprints": [
                        {
                            "kind": "sha256",
                            "value": "c" * 64,
                            "algorithm_version": "sha256-v1",
                        }
                    ],
                }
            ]
        },
    )

    assert response.status_code == 200
    assert response.json() == {
        "artifacts": [
            {
                "client_ref": "unknown",
                "status": "unresolved",
                "selected_artifact_id": None,
                "matches": [],
            }
        ]
    }


def test_batch_resolution_preserves_client_order(
    db_session: Session,
) -> None:
    artifact = seed_artifact(
        db_session,
        source_external_id="batch",
        creator_slug="batch-creator",
        creator_name="Batch Creator",
        mod_slug="batch-mod",
        mod_name="Batch Mod",
        version="1.0",
        filename="batch.package",
        size_bytes=100,
        fingerprints=[("sha256", "b" * 64, "sha256-v1")],
    )

    response = client.post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "first",
                    "fingerprints": [
                        {
                            "kind": "sha256",
                            "value": "b" * 64,
                            "algorithm_version": "sha256-v1",
                        }
                    ],
                },
                {
                    "client_ref": "second",
                    "fingerprints": [
                        {
                            "kind": "sha256",
                            "value": "0" * 64,
                            "algorithm_version": "sha256-v1",
                        }
                    ],
                },
            ]
        },
    )

    assert response.status_code == 200
    payload = response.json()["artifacts"]
    assert [item["client_ref"] for item in payload] == ["first", "second"]
    assert payload[0]["selected_artifact_id"] == str(artifact.id)
    assert payload[1]["status"] == "unresolved"
