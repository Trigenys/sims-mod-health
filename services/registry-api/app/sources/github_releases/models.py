from __future__ import annotations

from datetime import datetime

from pydantic import BaseModel, ConfigDict, Field


class GitHubModel(BaseModel):
    model_config = ConfigDict(extra="ignore")


class GitHubOwner(GitHubModel):
    id: int
    login: str
    type: str | None = None


class GitHubRepository(GitHubModel):
    id: int
    node_id: str | None = None
    name: str
    full_name: str
    private: bool
    html_url: str
    description: str | None = None
    owner: GitHubOwner


class GitHubReleaseAsset(GitHubModel):
    id: int
    name: str
    label: str | None = None
    content_type: str | None = None
    size: int = 0
    browser_download_url: str
    created_at: datetime | None = None
    updated_at: datetime | None = None
    download_count: int | None = None
    digest: str | None = None


class GitHubRelease(GitHubModel):
    id: int
    tag_name: str
    target_commitish: str | None = None
    name: str | None = None
    body: str | None = None
    draft: bool = False
    prerelease: bool = False
    created_at: datetime
    published_at: datetime | None = None
    html_url: str
    assets: list[GitHubReleaseAsset] = Field(default_factory=list)


class GitHubTagCommit(GitHubModel):
    sha: str
    url: str | None = None


class GitHubTag(GitHubModel):
    name: str
    zipball_url: str | None = None
    tarball_url: str | None = None
    commit: GitHubTagCommit
