from __future__ import annotations

from fastapi.testclient import TestClient
from sqlalchemy import func, select
from sqlalchemy.orm import Session

from app.domain import (
    Artifact,
    CompatibilityReport,
    ConflictRule,
    DependencyRule,
    Fingerprint,
    ModRelease,
    Source,
)
from app.main import app
from app.services.curseforge_sync import CurseForgeSyncService
from app.sources.curseforge.client import CurseForgeUnavailable
from app.sources.curseforge.models import CurseForgeFile, CurseForgeMod


class SuccessfulClient:
    def get_mod(self, mod_id: int) -> CurseForgeMod:
        return CurseForgeMod.model_validate(
            {
                "id": mod_id,
                "gameId": 99,
                "name": "Example Mod",
                "slug": "example-mod",
                "summary": "Summary",
                "authors": [{"id": 7, "name": "Creator"}],
                "links": {"websiteUrl": "https://example.test/mod"},
                "isAvailable": True,
            }
        )

    def get_mod_files(self, mod_id: int) -> list[CurseForgeFile]:
        return [
            CurseForgeFile.model_validate(
                {
                    "id": 1001,
                    "gameId": 99,
                    "modId": mod_id,
                    "isAvailable": True,
                    "displayName": "2.4.1",
                    "fileName": "Example.zip",
                    "releaseType": 1,
                    "fileStatus": 4,
                    "hashes": [
                        {"value": "AAAA", "algo": 1},
                        {"value": "BBBB", "algo": 2},
                    ],
                    "fileDate": "2026-09-01T00:00:00Z",
                    "fileLength": 1234,
                    "downloadUrl": "https://example.test/file",
                    "gameVersions": ["1.116.240"],
                    "dependencies": [
                        {"modId": 77, "relationType": 3},
                        {"modId": 88, "relationType": 5},
                    ],
                    "fileFingerprint": 987654321,
                    "modules": [],
                }
            )
        ]

    def get_file_changelog(self, mod_id: int, file_id: int) -> str:
        return "Fixed things"


class UnavailableClient:
    def get_mod(self, _mod_id: int):
        raise CurseForgeUnavailable(
            "temporary",
            status_code=429,
            retry_after_seconds=30,
        )


def test_sync_persists_provenance_source_ids_fingerprints_and_relationships(
    db_session: Session,
) -> None:
    result = CurseForgeSyncService(db_session, SuccessfulClient()).sync_mod(
        42,
        include_changelogs=True,
    )

    assert result.status == "updated"
    assert result.releases_upserted == 1
    assert result.artifacts_upserted == 1

    source = db_session.scalar(
        select(Source).where(Source.kind == "curseforge", Source.external_id == "42")
    )
    assert source is not None
    assert source.metadata_json["retrieved_at"]

    release = db_session.scalar(select(ModRelease))
    artifact = db_session.scalar(select(Artifact))
    assert release is not None and artifact is not None
    assert release.source_record_id == "1001"
    assert artifact.source_record_id == "1001"
    assert release.changelog == "Fixed things"
    assert artifact.metadata_json["curseforge"]["game_versions"] == ["1.116.240"]

    fingerprints = list(
        db_session.scalars(
            select(Fingerprint).order_by(Fingerprint.kind)
        )
    )
    assert [(item.kind, item.value) for item in fingerprints] == [
        ("curseforge", "987654321"),
        ("md5", "bbbb"),
        ("sha1", "aaaa"),
    ]

    dependency = db_session.scalar(select(DependencyRule))
    conflict = db_session.scalar(select(ConflictRule))
    assert dependency is not None
    assert dependency.release_id == release.id
    assert dependency.target_source_kind == "curseforge"
    assert dependency.target_source_external_id == "77"
    assert dependency.source_id == source.id
    assert dependency.retrieved_at is not None

    assert conflict is not None
    assert conflict.release_id == release.id
    assert conflict.target_source_kind == "curseforge"
    assert conflict.target_source_external_id == "88"
    assert conflict.source_id == source.id

    compatibility_count = db_session.scalar(
        select(func.count()).select_from(CompatibilityReport)
    )
    assert compatibility_count == 0


def test_reingestion_is_idempotent_for_release_artifact_and_relationships(
    db_session: Session,
) -> None:
    service = CurseForgeSyncService(db_session, SuccessfulClient())

    assert service.sync_mod(42).status == "updated"
    assert service.sync_mod(42).status == "updated"

    assert db_session.scalar(select(func.count()).select_from(ModRelease)) == 1
    assert db_session.scalar(select(func.count()).select_from(Artifact)) == 1
    assert db_session.scalar(select(func.count()).select_from(Fingerprint)) == 3
    assert db_session.scalar(select(func.count()).select_from(DependencyRule)) == 1
    assert db_session.scalar(select(func.count()).select_from(ConflictRule)) == 1


def test_unavailable_source_is_isolated_and_creates_no_health_claim(
    db_session: Session,
) -> None:
    result = CurseForgeSyncService(db_session, UnavailableClient()).sync_mod(42)

    assert result.status == "unavailable"
    assert result.retry_after_seconds == 30
    assert db_session.scalar(select(func.count()).select_from(Source)) == 0
    assert db_session.scalar(select(func.count()).select_from(CompatibilityReport)) == 0
    assert db_session.scalar(select(func.count()).select_from(DependencyRule)) == 0
    assert db_session.scalar(select(func.count()).select_from(ConflictRule)) == 0

    response = TestClient(app).post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "still-works",
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
    assert response.json()["artifacts"][0]["matches"] == []
