from __future__ import annotations

import httpx
import pytest

from app.sources.curseforge.client import CurseForgeClient, CurseForgeUnavailable


def test_client_sends_api_key_and_reads_mod() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        assert request.url.host == "api.curseforge.com"
        assert request.url.path == "/v1/mods/42"
        assert request.headers["x-api-key"] == "secret"
        return httpx.Response(
            200,
            json={
                "data": {
                    "id": 42,
                    "gameId": 99,
                    "name": "Example",
                    "slug": "example",
                    "authors": [],
                }
            },
        )

    client = CurseForgeClient(
        "secret",
        transport=httpx.MockTransport(handler),
        max_retries=0,
    )
    try:
        mod = client.get_mod(42)
    finally:
        client.close()

    assert mod.id == 42
    assert mod.slug == "example"


def test_rate_limit_honors_retry_after_then_recovers() -> None:
    calls = 0
    sleeps: list[float] = []

    def handler(_request: httpx.Request) -> httpx.Response:
        nonlocal calls
        calls += 1
        if calls == 1:
            return httpx.Response(429, headers={"Retry-After": "2"})
        return httpx.Response(
            200,
            json={
                "data": {
                    "id": 42,
                    "gameId": 99,
                    "name": "Example",
                    "slug": "example",
                    "authors": [],
                }
            },
        )

    client = CurseForgeClient(
        "secret",
        transport=httpx.MockTransport(handler),
        sleep=sleeps.append,
        max_retries=1,
        max_backoff_seconds=8,
    )
    try:
        assert client.get_mod(42).id == 42
    finally:
        client.close()

    assert calls == 2
    assert sleeps == [2.0]


def test_retry_after_sleep_is_bounded() -> None:
    sleeps: list[float] = []

    def handler(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(429, headers={"Retry-After": "600"})

    client = CurseForgeClient(
        "secret",
        transport=httpx.MockTransport(handler),
        sleep=sleeps.append,
        max_retries=1,
        max_backoff_seconds=3,
    )
    try:
        with pytest.raises(CurseForgeUnavailable) as caught:
            client.get_mod(42)
    finally:
        client.close()

    assert sleeps == [3]
    assert caught.value.retry_after_seconds == 600.0


def test_server_failure_exhausts_retry_budget_without_leaking_key() -> None:
    sleeps: list[float] = []

    def handler(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(503)

    client = CurseForgeClient(
        "top-secret",
        transport=httpx.MockTransport(handler),
        sleep=sleeps.append,
        max_retries=2,
        base_backoff_seconds=0.25,
    )
    try:
        with pytest.raises(CurseForgeUnavailable) as caught:
            client.get_mod(42)
    finally:
        client.close()

    assert sleeps == [0.25, 0.5]
    assert "top-secret" not in str(caught.value)


def test_files_pagination_combines_pages() -> None:
    indices: list[int] = []

    def file_payload(file_id: int) -> dict:
        return {
            "id": file_id,
            "gameId": 99,
            "modId": 42,
            "isAvailable": True,
            "displayName": f"Release {file_id}",
            "fileName": f"release-{file_id}.zip",
            "releaseType": 1,
            "fileStatus": 4,
            "hashes": [],
            "fileDate": "2026-09-01T00:00:00Z",
            "fileLength": 100,
            "gameVersions": ["1.116"],
            "dependencies": [],
            "modules": [],
        }

    def handler(request: httpx.Request) -> httpx.Response:
        index = int(request.url.params["index"])
        indices.append(index)
        if index == 0:
            data = [file_payload(1), file_payload(2)]
            pagination = {"index": 0, "pageSize": 50, "resultCount": 2, "totalCount": 3}
        else:
            data = [file_payload(3)]
            pagination = {"index": 2, "pageSize": 50, "resultCount": 1, "totalCount": 3}
        return httpx.Response(200, json={"data": data, "pagination": pagination})

    client = CurseForgeClient(
        "secret",
        transport=httpx.MockTransport(handler),
        max_retries=0,
    )
    try:
        files = client.get_mod_files(42)
    finally:
        client.close()

    assert [item.id for item in files] == [1, 2, 3]
    assert indices == [0, 2]
