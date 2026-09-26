from __future__ import annotations

from datetime import datetime, timezone

import httpx
import pytest

from app.sources.github_releases.client import (
    GitHubReleasesClient,
    GitHubReleasesUnavailable,
    GitHubRepositoryNotPublic,
)


def _repository_payload(*, private: bool = False) -> dict:
    return {
        "id": 101,
        "node_id": "R_repo",
        "name": "example-mod",
        "full_name": "creator/example-mod",
        "private": private,
        "html_url": "https://github.com/creator/example-mod",
        "description": "Example mod",
        "owner": {"id": 7, "login": "creator", "type": "User"},
    }


def _release_payload(release_id: int) -> dict:
    return {
        "id": release_id,
        "tag_name": f"v{release_id}",
        "target_commitish": "main",
        "name": f"Release {release_id}",
        "body": "Changes",
        "draft": False,
        "prerelease": False,
        "created_at": "2026-09-01T00:00:00Z",
        "published_at": "2026-09-01T01:00:00Z",
        "html_url": f"https://github.com/creator/example-mod/releases/tag/v{release_id}",
        "assets": [],
    }


def test_public_repository_works_without_authentication() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        assert request.url.path == "/repos/creator/example-mod"
        assert "Authorization" not in request.headers
        return httpx.Response(200, json=_repository_payload())

    client = GitHubReleasesClient(
        transport=httpx.MockTransport(handler),
        max_retries=0,
    )
    try:
        repository = client.get_repository("creator", "example-mod")
    finally:
        client.close()

    assert repository.full_name == "creator/example-mod"
    assert repository.private is False


def test_optional_server_token_is_sent_but_not_leaked() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        assert request.headers["Authorization"] == "Bearer top-secret"
        return httpx.Response(503)

    client = GitHubReleasesClient(
        "top-secret",
        transport=httpx.MockTransport(handler),
        max_retries=0,
    )
    try:
        with pytest.raises(GitHubReleasesUnavailable) as caught:
            client.get_repository("creator", "example-mod")
    finally:
        client.close()

    assert "top-secret" not in str(caught.value)


def test_authenticated_private_repository_is_rejected() -> None:
    client = GitHubReleasesClient(
        "server-token",
        transport=httpx.MockTransport(
            lambda _request: httpx.Response(
                200,
                json=_repository_payload(private=True),
            )
        ),
        max_retries=0,
    )
    try:
        with pytest.raises(GitHubRepositoryNotPublic):
            client.get_repository("creator", "example-mod")
    finally:
        client.close()


def test_release_pagination_combines_pages() -> None:
    pages: list[int] = []

    def handler(request: httpx.Request) -> httpx.Response:
        page = int(request.url.params["page"])
        pages.append(page)
        if page == 1:
            return httpx.Response(
                200,
                json=[_release_payload(index) for index in range(1, 101)],
            )
        return httpx.Response(200, json=[_release_payload(101)])

    client = GitHubReleasesClient(
        transport=httpx.MockTransport(handler),
        max_retries=0,
    )
    try:
        releases = client.get_releases("creator", "example-mod")
    finally:
        client.close()

    assert len(releases) == 101
    assert pages == [1, 2]


def test_rate_limit_uses_reset_header_and_recovers() -> None:
    calls = 0
    sleeps: list[float] = []
    now = datetime(2026, 9, 26, 12, 0, tzinfo=timezone.utc)
    reset = int(now.timestamp()) + 3

    def handler(_request: httpx.Request) -> httpx.Response:
        nonlocal calls
        calls += 1
        if calls == 1:
            return httpx.Response(
                403,
                headers={
                    "X-RateLimit-Remaining": "0",
                    "X-RateLimit-Reset": str(reset),
                },
            )
        return httpx.Response(200, json=_repository_payload())

    client = GitHubReleasesClient(
        transport=httpx.MockTransport(handler),
        sleep=sleeps.append,
        now=lambda: now,
        max_retries=1,
        max_backoff_seconds=8,
    )
    try:
        assert client.get_repository("creator", "example-mod").id == 101
    finally:
        client.close()

    assert sleeps == [3.0]
    assert calls == 2
