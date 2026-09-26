from __future__ import annotations

import email.utils
import time
from collections.abc import Callable
from datetime import datetime, timezone
from typing import Any

import httpx

from app.sources.curseforge.models import (
    CurseForgeFile,
    CurseForgeMod,
    FilesResponse,
    ModResponse,
    StringResponse,
)


class CurseForgeError(RuntimeError):
    pass


class CurseForgeConfigurationError(CurseForgeError):
    pass


class CurseForgeNotFound(CurseForgeError):
    pass


class CurseForgeUnavailable(CurseForgeError):
    def __init__(
        self,
        message: str,
        *,
        status_code: int | None = None,
        retry_after_seconds: float | None = None,
    ) -> None:
        super().__init__(message)
        self.status_code = status_code
        self.retry_after_seconds = retry_after_seconds


class CurseForgeClient:
    OFFICIAL_BASE_URL = "https://api.curseforge.com"
    PAGE_SIZE = 50
    MAX_RESULTS = 10_000
    RETRYABLE_STATUS = frozenset({408, 429, 500, 502, 503, 504})

    def __init__(
        self,
        api_key: str | None,
        *,
        base_url: str = OFFICIAL_BASE_URL,
        timeout_seconds: float = 10.0,
        max_retries: int = 3,
        base_backoff_seconds: float = 0.25,
        max_backoff_seconds: float = 8.0,
        transport: httpx.BaseTransport | None = None,
        sleep: Callable[[float], None] = time.sleep,
        now: Callable[[], datetime] = lambda: datetime.now(timezone.utc),
    ) -> None:
        if not api_key:
            raise CurseForgeConfigurationError(
                "CURSEFORGE_API_KEY is required for the official CurseForge API."
            )
        if max_retries < 0:
            raise ValueError("max_retries must be >= 0")

        self._max_retries = max_retries
        self._base_backoff_seconds = base_backoff_seconds
        self._max_backoff_seconds = max_backoff_seconds
        self._sleep = sleep
        self._now = now
        self._client = httpx.Client(
            base_url=base_url.rstrip("/"),
            timeout=timeout_seconds,
            transport=transport,
            headers={
                "Accept": "application/json",
                "x-api-key": api_key,
                "User-Agent": "SimsModHealth-Registry/0.1",
            },
        )

    def close(self) -> None:
        self._client.close()

    def __enter__(self) -> "CurseForgeClient":
        return self

    def __exit__(self, *_args: object) -> None:
        self.close()

    def get_mod(self, mod_id: int) -> CurseForgeMod:
        payload = self._request_json("GET", f"/v1/mods/{mod_id}")
        return ModResponse.model_validate(payload).data

    def get_mod_files(self, mod_id: int) -> list[CurseForgeFile]:
        files: list[CurseForgeFile] = []
        index = 0

        while index < self.MAX_RESULTS:
            payload = self._request_json(
                "GET",
                f"/v1/mods/{mod_id}/files",
                params={"index": index, "pageSize": self.PAGE_SIZE},
            )
            page = FilesResponse.model_validate(payload)
            files.extend(page.data)

            if page.pagination.resultCount <= 0:
                break

            next_index = page.pagination.index + page.pagination.resultCount
            if next_index >= page.pagination.totalCount:
                break
            if next_index <= index:
                raise CurseForgeUnavailable(
                    "CurseForge pagination did not advance; aborting safely."
                )
            index = next_index

        return files

    def get_file_changelog(self, mod_id: int, file_id: int) -> str:
        payload = self._request_json(
            "GET",
            f"/v1/mods/{mod_id}/files/{file_id}/changelog",
        )
        return StringResponse.model_validate(payload).data

    def _request_json(
        self,
        method: str,
        path: str,
        *,
        params: dict[str, Any] | None = None,
        json: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        last_error: Exception | None = None

        for attempt in range(self._max_retries + 1):
            try:
                response = self._client.request(method, path, params=params, json=json)
            except httpx.RequestError as error:
                last_error = error
                if attempt >= self._max_retries:
                    raise CurseForgeUnavailable(
                        "CurseForge is currently unreachable."
                    ) from error
                self._sleep(self._backoff(attempt))
                continue

            if response.status_code == 404:
                raise CurseForgeNotFound(f"CurseForge resource not found: {path}")

            if response.status_code in self.RETRYABLE_STATUS:
                retry_after = self._retry_after_seconds(response)
                if attempt >= self._max_retries:
                    raise CurseForgeUnavailable(
                        "CurseForge is temporarily unavailable or rate limited.",
                        status_code=response.status_code,
                        retry_after_seconds=retry_after,
                    )
                self._sleep(retry_after if retry_after is not None else self._backoff(attempt))
                continue

            if response.is_error:
                raise CurseForgeError(
                    f"CurseForge request failed with HTTP {response.status_code}."
                )

            try:
                data = response.json()
            except ValueError as error:
                raise CurseForgeUnavailable(
                    "CurseForge returned an invalid JSON response.",
                    status_code=response.status_code,
                ) from error

            if not isinstance(data, dict):
                raise CurseForgeUnavailable(
                    "CurseForge returned an unexpected response shape.",
                    status_code=response.status_code,
                )
            return data

        raise CurseForgeUnavailable("CurseForge request failed.") from last_error

    def _backoff(self, attempt: int) -> float:
        return min(
            self._base_backoff_seconds * (2**attempt),
            self._max_backoff_seconds,
        )

    def _retry_after_seconds(self, response: httpx.Response) -> float | None:
        value = response.headers.get("Retry-After")
        if not value:
            return None

        try:
            return max(0.0, float(value))
        except ValueError:
            try:
                retry_at = email.utils.parsedate_to_datetime(value)
            except (TypeError, ValueError):
                return None

            if retry_at.tzinfo is None:
                retry_at = retry_at.replace(tzinfo=timezone.utc)
            return max(0.0, (retry_at - self._now()).total_seconds())
