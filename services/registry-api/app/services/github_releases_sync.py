from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Literal

from sqlalchemy.orm import Session

from app.repositories.github_releases_ingestion import (
    GitHubReleasesIngestionRepository,
)
from app.sources.github_releases.client import (
    GitHubReleasesClient,
    GitHubReleasesNotFound,
    GitHubReleasesUnavailable,
    GitHubRepositoryNotPublic,
)
from app.sources.github_releases.mapper import GitHubReleasesMapper


@dataclass(frozen=True, slots=True)
class GitHubReleasesSyncResult:
    status: Literal["updated", "not_found", "not_public", "unavailable"]
    repository: str
    releases_upserted: int = 0
    artifacts_upserted: int = 0
    retrieved_at: datetime | None = None
    retry_after_seconds: float | None = None


class GitHubReleasesSyncService:
    def __init__(
        self,
        session: Session,
        client: GitHubReleasesClient,
        *,
        mapper: GitHubReleasesMapper | None = None,
    ) -> None:
        self._session = session
        self._client = client
        self._mapper = mapper or GitHubReleasesMapper()
        self._repository = GitHubReleasesIngestionRepository(session)

    def sync_repository(self, owner: str, repo: str) -> GitHubReleasesSyncResult:
        repository_name = f"{owner}/{repo}"
        retrieved_at = datetime.now(timezone.utc)

        try:
            repository = self._client.get_repository(owner, repo)
            releases = self._client.get_releases(owner, repo)
            tags = self._client.get_tags(owner, repo)
        except GitHubReleasesNotFound:
            self._session.rollback()
            return GitHubReleasesSyncResult(
                status="not_found",
                repository=repository_name,
            )
        except GitHubRepositoryNotPublic:
            self._session.rollback()
            return GitHubReleasesSyncResult(
                status="not_public",
                repository=repository_name,
            )
        except GitHubReleasesUnavailable as error:
            self._session.rollback()
            return GitHubReleasesSyncResult(
                status="unavailable",
                repository=repository_name,
                retry_after_seconds=error.retry_after_seconds,
            )

        project = self._mapper.map_project(
            repository,
            releases,
            tags,
            retrieved_at=retrieved_at,
        )

        try:
            releases_upserted, artifacts_upserted = self._repository.upsert_project(
                project,
                retrieved_at=retrieved_at,
            )
            self._session.commit()
        except Exception:
            self._session.rollback()
            raise

        return GitHubReleasesSyncResult(
            status="updated",
            repository=repository.full_name,
            releases_upserted=releases_upserted,
            artifacts_upserted=artifacts_upserted,
            retrieved_at=retrieved_at,
        )
