from __future__ import annotations

from datetime import datetime, timezone

from app.sources.github_releases.mapper import GitHubReleasesMapper
from app.sources.github_releases.models import (
    GitHubRelease,
    GitHubRepository,
    GitHubTag,
)


def test_mapper_preserves_release_tag_asset_and_digest_provenance() -> None:
    repository = GitHubRepository.model_validate(
        {
            "id": 101,
            "node_id": "R_repo",
            "name": "Example-Mod",
            "full_name": "Creator/Example-Mod",
            "private": False,
            "html_url": "https://github.com/Creator/Example-Mod",
            "description": "Example mod",
            "owner": {"id": 7, "login": "Creator", "type": "User"},
        }
    )
    release = GitHubRelease.model_validate(
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
            "html_url": "https://github.com/Creator/Example-Mod/releases/tag/v2.4.1",
            "assets": [
                {
                    "id": 9001,
                    "name": "ExampleMod.ts4script",
                    "content_type": "application/octet-stream",
                    "size": 1234,
                    "browser_download_url": "https://github.com/Creator/Example-Mod/releases/download/v2.4.1/ExampleMod.ts4script",
                    "digest": "sha256:ABCDEF",
                    "download_count": 12,
                }
            ],
        }
    )
    tag = GitHubTag.model_validate(
        {
            "name": "v2.4.1",
            "zipball_url": "https://api.github.com/repos/Creator/Example-Mod/zipball/v2.4.1",
            "tarball_url": "https://api.github.com/repos/Creator/Example-Mod/tarball/v2.4.1",
            "commit": {"sha": "deadbeef"},
        }
    )

    mapped = GitHubReleasesMapper().map_project(
        repository,
        [release],
        [tag],
        retrieved_at=datetime(2026, 9, 26, tzinfo=timezone.utc),
    )

    assert mapped.source_external_id == "101"
    assert mapped.creator_slug == "github-owner-7"
    assert mapped.mod_slug == "example-mod"
    assert mapped.releases[0].version == "v2.4.1"
    assert mapped.releases[0].changelog == "Fixed things"
    assert mapped.releases[0].artifacts[0].artifact_kind == "ts4script"
    assert [
        (fingerprint.kind, fingerprint.value, fingerprint.algorithm_version)
        for fingerprint in mapped.releases[0].artifacts[0].fingerprints
    ] == [
        ("sha256", "abcdef", "sha256-github-asset-digest-v1")
    ]
    assert mapped.source_metadata["tags"] == [
        {
            "name": "v2.4.1",
            "commit_sha": "deadbeef",
            "zipball_url": "https://api.github.com/repos/Creator/Example-Mod/zipball/v2.4.1",
            "tarball_url": "https://api.github.com/repos/Creator/Example-Mod/tarball/v2.4.1",
        }
    ]


def test_mapper_excludes_draft_releases() -> None:
    repository = GitHubRepository.model_validate(
        {
            "id": 101,
            "name": "example-mod",
            "full_name": "creator/example-mod",
            "private": False,
            "html_url": "https://github.com/creator/example-mod",
            "owner": {"id": 7, "login": "creator"},
        }
    )
    draft = GitHubRelease.model_validate(
        {
            "id": 502,
            "tag_name": "v-next",
            "draft": True,
            "created_at": "2026-09-01T00:00:00Z",
            "html_url": "https://github.com/creator/example-mod/releases/tag/v-next",
            "assets": [],
        }
    )

    mapped = GitHubReleasesMapper().map_project(
        repository,
        [draft],
        [],
        retrieved_at=datetime(2026, 9, 26, tzinfo=timezone.utc),
    )

    assert mapped.releases == ()
