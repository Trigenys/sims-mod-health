from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any

from app.sources.github_releases.models import (
    GitHubRelease,
    GitHubReleaseAsset,
    GitHubRepository,
    GitHubTag,
)


@dataclass(frozen=True, slots=True)
class MappedFingerprint:
    kind: str
    value: str
    algorithm_version: str


@dataclass(frozen=True, slots=True)
class MappedArtifact:
    source_record_id: str
    filename: str
    artifact_kind: str
    size_bytes: int
    source_url: str
    fingerprints: tuple[MappedFingerprint, ...]
    metadata: dict[str, Any]


@dataclass(frozen=True, slots=True)
class MappedRelease:
    source_record_id: str
    version: str
    released_at: datetime
    source_url: str
    changelog: str | None
    metadata: dict[str, Any]
    artifacts: tuple[MappedArtifact, ...]


@dataclass(frozen=True, slots=True)
class MappedProject:
    source_external_id: str
    source_url: str
    creator_slug: str
    creator_name: str
    mod_slug: str
    mod_name: str
    summary: str | None
    source_metadata: dict[str, Any]
    releases: tuple[MappedRelease, ...]


class GitHubReleasesMapper:
    def map_project(
        self,
        repository: GitHubRepository,
        releases: list[GitHubRelease],
        tags: list[GitHubTag],
        *,
        retrieved_at: datetime,
    ) -> MappedProject:
        public_releases = [release for release in releases if not release.draft]
        mapped_releases = tuple(
            self.map_release(release)
            for release in sorted(
                public_releases,
                key=lambda item: (item.published_at or item.created_at, item.id),
            )
        )

        return MappedProject(
            source_external_id=str(repository.id),
            source_url=repository.html_url,
            creator_slug=f"github-owner-{repository.owner.id}",
            creator_name=repository.owner.login,
            mod_slug=repository.name.lower(),
            mod_name=repository.name,
            summary=repository.description,
            source_metadata={
                "repository_id": repository.id,
                "repository_node_id": repository.node_id,
                "repository_full_name": repository.full_name,
                "owner": {
                    "id": repository.owner.id,
                    "login": repository.owner.login,
                    "type": repository.owner.type,
                },
                "private": repository.private,
                "tags": [
                    {
                        "name": tag.name,
                        "commit_sha": tag.commit.sha,
                        "zipball_url": tag.zipball_url,
                        "tarball_url": tag.tarball_url,
                    }
                    for tag in tags
                ],
                "retrieved_at": retrieved_at.isoformat(),
            },
            releases=mapped_releases,
        )

    def map_release(self, release: GitHubRelease) -> MappedRelease:
        return MappedRelease(
            source_record_id=str(release.id),
            version=release.tag_name or release.name or str(release.id),
            released_at=release.published_at or release.created_at,
            source_url=release.html_url,
            changelog=release.body,
            metadata={
                "github": {
                    "release_id": release.id,
                    "tag_name": release.tag_name,
                    "target_commitish": release.target_commitish,
                    "release_name": release.name,
                    "draft": release.draft,
                    "prerelease": release.prerelease,
                    "created_at": release.created_at.isoformat(),
                    "published_at": (
                        release.published_at.isoformat()
                        if release.published_at is not None
                        else None
                    ),
                }
            },
            artifacts=tuple(self.map_asset(release.id, asset) for asset in release.assets),
        )

    def map_asset(
        self,
        release_id: int,
        asset: GitHubReleaseAsset,
    ) -> MappedArtifact:
        fingerprints: list[MappedFingerprint] = []
        if asset.digest:
            algorithm, separator, value = asset.digest.partition(":")
            if separator and algorithm.lower() == "sha256" and value:
                fingerprints.append(
                    MappedFingerprint(
                        kind="sha256",
                        value=value.lower(),
                        algorithm_version="sha256-v1",
                    )
                )

        return MappedArtifact(
            source_record_id=str(asset.id),
            filename=asset.name,
            artifact_kind=_artifact_kind(asset.name),
            size_bytes=asset.size,
            source_url=asset.browser_download_url,
            fingerprints=tuple(fingerprints),
            metadata={
                "github": {
                    "release_id": release_id,
                    "asset_id": asset.id,
                    "label": asset.label,
                    "content_type": asset.content_type,
                    "digest": asset.digest,
                    "created_at": (
                        asset.created_at.isoformat()
                        if asset.created_at is not None
                        else None
                    ),
                    "updated_at": (
                        asset.updated_at.isoformat()
                        if asset.updated_at is not None
                        else None
                    ),
                    "download_count": asset.download_count,
                }
            },
        )


def _artifact_kind(filename: str) -> str:
    suffix = Path(filename).suffix.lower()
    if suffix == ".package":
        return "package"
    if suffix == ".ts4script":
        return "ts4script"
    if suffix in {".zip", ".rar", ".7z"}:
        return "archive"
    return "unknown"
