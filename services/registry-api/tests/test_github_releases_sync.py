from __future__ import annotations

from sqlalchemy import func, select
from sqlalchemy.orm import Session

from app.domain import Artifact, CompatibilityReport, Fingerprint, ModRelease, Source
from app.services.github_releases_sync import GitHubReleasesSyncService
from app.sources.github_releases.client import (
    GitHubReleasesUnavailable,
    GitHubRepositoryNotPublic,
)
from app.sources.github_releases.models import (
    GitHubRelease,
    GitHubRepository,
    GitHubTag,
)


class SuccessfulClient:
    def get_repository(self, owner: str, repo: str) -> GitHubRepository:
        return GitHubRepository.model_validate(
            {
                "id": 101,
                "node_id": "R_repo",
                "name": repo,
                "full_name": f"{owner}/{repo}",
                "private": False,
                "html_url": f"https://github.com/{owner}/{repo}",
                "description": "Example mod",
                "owner": {"id": 7, "login": owner, "type": "User"},
            }
        )

    def get_releases(self, owner: str, repo: str) -> list[GitHubRelease]:
        return [
            GitHubRelease.model_validate(
                {
                    "id": 501,
                    "tag_name": "v2.4.1",
                    "target_commitish": "main",
                    "name": "2.4.1",
                    "body": "Fixed things",
                    "draft": False,
                    "prerelease": False,
                    "created_at": "2026-09-01T00:00:00Z",
                    "published_at": "2026-09-01T01:00:00Z",
                    "html_url": f"https://github.com/{owner}/{repo}/releases/tag/v2.4.1",
                    "assets": [
                        {
                            "id": 9001,
                            "name": "ExampleMod.zip",
                            "content_type": "application/zip",
                            "size": 1234,
                            "browser_download_url": f"https://github.com/{owner}/{repo}/releases/download/v2.4.1/ExampleMod.zip",
                            "digest": "sha256:AAAA",
                        }
                    ],
                }
            ),
            GitHubRelease.model_validate(
                {
                    "id": 502,
                    "tag_name": "v2.5.0-notes-only",
                    "body": "Metadata-only release",
                    "draft": False,
                    "prerelease": True,
                    "created_at": "2026-09-02T00:00:00Z",
                    "html_url": f"https://github.com/{owner}/{repo}/releases/tag/v2.5.0-notes-only",
                    "assets": [],
                }
            ),
        ]

    def get_tags(self, owner: str, repo: str) -> list[GitHubTag]:
        return [
            GitHubTag.model_validate(
                {
                    "name": "v2.4.1",
                    "commit": {"sha": "deadbeef"},
                }
            )
        ]


class UnavailableClient:
    def get_repository(self, _owner: str, _repo: str):
        raise GitHubReleasesUnavailable(
            "temporary",
            status_code=429,
            retry_after_seconds=30,
        )


class PrivateClient:
    def get_repository(self, _owner: str, _repo: str):
        raise GitHubRepositoryNotPublic("private")


def test_sync_persists_public_release_asset_and_provenance(
    db_session: Session,
) -> None:
    result = GitHubReleasesSyncService(
        db_session,
        SuccessfulClient(),
    ).sync_repository("creator", "example-mod")

    assert result.status == "updated"
    assert result.releases_upserted == 2
    assert result.artifacts_upserted == 1

    source = db_session.scalar(
        select(Source).where(
            Source.kind == "github_releases",
            Source.external_id == "101",
        )
    )
    assert source is not None
    assert source.metadata_json["repository_full_name"] == "creator/example-mod"
    assert source.metadata_json["tags"][0]["commit_sha"] == "deadbeef"

    releases = list(db_session.scalars(select(ModRelease).order_by(ModRelease.version)))
    artifact = db_session.scalar(select(Artifact))
    fingerprint = db_session.scalar(select(Fingerprint))
    assert len(releases) == 2
    assert artifact is not None and fingerprint is not None
    assert artifact.source_record_id == "9001"
    assert artifact.metadata_json["github"]["release_id"] == 501
    assert fingerprint.kind == "sha256"
    assert fingerprint.value == "aaaa"
    assert fingerprint.algorithm_version == "sha256-github-asset-digest-v1"

    assert db_session.scalar(
        select(func.count()).select_from(CompatibilityReport)
    ) == 0


def test_reingestion_is_idempotent_for_release_and_asset(
    db_session: Session,
) -> None:
    service = GitHubReleasesSyncService(db_session, SuccessfulClient())

    assert service.sync_repository("creator", "example-mod").status == "updated"
    assert service.sync_repository("creator", "example-mod").status == "updated"

    assert db_session.scalar(select(func.count()).select_from(ModRelease)) == 2
    assert db_session.scalar(select(func.count()).select_from(Artifact)) == 1
    assert db_session.scalar(select(func.count()).select_from(Fingerprint)) == 1


def test_unavailable_source_is_explicit_and_creates_no_health_claim(
    db_session: Session,
) -> None:
    result = GitHubReleasesSyncService(
        db_session,
        UnavailableClient(),
    ).sync_repository("creator", "example-mod")

    assert result.status == "unavailable"
    assert result.retry_after_seconds == 30
    assert db_session.scalar(select(func.count()).select_from(Source)) == 0
    assert db_session.scalar(
        select(func.count()).select_from(CompatibilityReport)
    ) == 0


def test_private_repository_is_refused(
    db_session: Session,
) -> None:
    result = GitHubReleasesSyncService(
        db_session,
        PrivateClient(),
    ).sync_repository("creator", "private-mod")

    assert result.status == "not_public"
    assert db_session.scalar(select(func.count()).select_from(Source)) == 0
