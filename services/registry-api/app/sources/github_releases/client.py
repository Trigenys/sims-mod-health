from __future__ import annotations

import email.utils
import time
from collections.abc import Callable
from datetime import datetime, timezone
from typing import Any

import httpx

from app.sources.github_releases.models import (
    GitHubRelease,
    GitHubRepository,
    GitHubTag,
)


class GitHubReleasesError(RuntimeError):
    pass


class GitHubReleasesNotFound(GitHubReleasesError):
    pass


class GitHubRepositoryNotPublic(GitHubReleasesError):
    pass


class GitHubReleasesUnavailable(GitHubReleasesError):
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


class GitHubReleasesClient:
    OFFICIAL_BASE_URL = "https://api.github.com"
    PAGE_SIZE = 100
    MAX_RESULTS = 10_000
    RETRYABLE_STATUS = frozenset({408, 429, 500, 502, 503, 504})

    def __init__(
        self,
        token: str | None = None,
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
        if max_retries < 0:
            raise ValueError("max_retries must be >= 0")

        headers = {
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "SimsModHealth-Registry/0.1",
        }
        if token:
            headers["Authorization"] = f"Bearer {token}"

        self._token = token
        self._max_retries = max_retries
        self._base_backoff_seconds = base_backoff_seconds
        self._max_backoff_seconds = max_backoff_seconds
        self._sleep = sleep
        self._now = now
        self._client = httpx.Client(
            base_url=base_url.rstrip("/"),
            timeout=timeout_seconds,
            transport=transport,
            headers=headers,
        )

    def close(self) -> None:
        self._client.close()

    def __enter__(self) -> "GitHubReleasesClient":
        return self

    def __exit__(self, *_args: object) -> None:
        self.close()

    def get_repository(self, owner: str, repo: str) -> GitHubRepository:
        payload = self._request_json("GET", self._repo_path(owner, repo))
        repository = GitHubRepository.model_validate(payload)
        if repository.private:
            raise GitHubRepositoryNotPublic(
                f"GitHub repository {repository.full_name} is not public."
            )
        return repository

    def get_releases(self, owner: str, repo: str) -> list[GitHubRelease]:
        payloads = self._paginated(owner, repo, "releases")
        return [GitHubRelease.model_validate(item) for item in payloads]

    def get_tags(self, owner: str, repo: str) -> list[GitHubTag]:
        payloads = self._paginated(owner, repo, "tags")
        return [GitHubTag.model_validate(item) for item in payloads]

    def _paginated(self, owner: str, repo: str, resource: str) -> list[dict[str, Any]]:
        items: list[dict[str, Any]] = []
        page = 1

        while len(items) < self.MAX_RESULTS:
            payload = self._request_json(
                "GET",
                f"{self._repo_path(owner, repo)}/{resource}",
                params={"per_page": self.PAGE_SIZE, "page": page},
            )
            if not isinstance(payload, list):
                raise GitHubReleasesUnavailable(
                    f"GitHub {resource} returned an unexpected response shape."
                )

            typed = [item for item in payload if isinstance(item, dict)]
            items.extend(typed)
            if len(payload) < self.PAGE_SIZE:
                break
            page += 1

        return items[: self.MAX_RESULTS]

    def _repo_path(self, owner: str, repo: str) -> str:
        owner = owner.strip()
        repo = repo.strip()
        if not owner or not repo or "/" in owner or "/" in repo:
            raise ValueError("owner and repo must be non-empty GitHub path segments")
        return f"/repos/{owner}/{repo}"

    def _request_json(
        self,
        method: str,
        path: str,
        *,
        params: dict[str, Any] | None = None,
    ) -> Any:
        last_error: Exception | None = None

        for attempt in range(self._max_retries + 1):
            try:
                response = self._client.request(method, path, params=params)
            except httpx.RequestError as error:
                last_error = error
                if attempt >= self._max_retries:
                    raise GitHubReleasesUnavailable(
                        "GitHub Releases is currently unreachable."
                    ) from error
                self._sleep(self._backoff(attempt))
                continue

            if response.status_code == 404:
                raise GitHubReleasesNotFound(f"GitHub resource not found: {path}")

            rate_limited = (
                response.status_code == 403
                and response.headers.get("X-RateLimit-Remaining") == "0"
            )
            if response.status_code in self.RETRYABLE_STATUS or rate_limited:
                retry_after = self._retry_after_seconds(response)
                if attempt >= self._max_retries:
                    raise GitHubReleasesUnavailable(
                        "GitHub is temporarily unavailable or rate limited.",
                        status_code=response.status_code,
                        retry_after_seconds=retry_after,
                    )
                delay = retry_after if retry_after is not None else self._backoff(attempt)
                self._sleep(min(delay, self._max_backoff_seconds))
                continue

            if response.is_error:
                raise GitHubReleasesError(
                    f"GitHub request failed with HTTP {response.status_code}."
                )

            try:
                return response.json()
            except ValueError as error:
                raise GitHubReleasesUnavailable(
                    "GitHub returned an invalid JSON response.",
                    status_code=response.status_code,
                ) from error

        raise GitHubReleasesUnavailable("GitHub request failed.") from last_error

    def _backoff(self, attempt: int) -> float:
        return min(
            self._base_backoff_seconds * (2**attempt),
            self._max_backoff_seconds,
        )

    def _retry_after_seconds(self, response: httpx.Response) -> float | None:
        retry_after = response.headers.get("Retry-After")
        if retry_after:
            try:
                return max(0.0, float(retry_after))
            except ValueError:
                try:
                    retry_at = email.utils.parsedate_to_datetime(retry_after)
                except (TypeError, ValueError):
                    retry_at = None
                if retry_at is not None:
                    if retry_at.tzinfo is None:
                        retry_at = retry_at.replace(tzinfo=timezone.utc)
                    return max(0.0, (retry_at - self._now()).total_seconds())

        reset = response.headers.get("X-RateLimit-Reset")
        if reset:
            try:
                reset_at = datetime.fromtimestamp(float(reset), tz=timezone.utc)
            except (ValueError, OSError, OverflowError):
                return None
            return max(0.0, (reset_at - self._now()).total_seconds())

        return None
