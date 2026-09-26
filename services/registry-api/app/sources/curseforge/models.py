from __future__ import annotations

from datetime import datetime

from pydantic import BaseModel, ConfigDict, Field


class CurseForgeModel(BaseModel):
    model_config = ConfigDict(extra="ignore")


class CurseForgeAuthor(CurseForgeModel):
    id: int
    name: str
    url: str | None = None


class CurseForgeLinks(CurseForgeModel):
    websiteUrl: str | None = None
    wikiUrl: str | None = None
    issuesUrl: str | None = None
    sourceUrl: str | None = None


class CurseForgeMod(CurseForgeModel):
    id: int
    gameId: int
    name: str
    slug: str
    summary: str | None = None
    status: int | None = None
    authors: list[CurseForgeAuthor] = Field(default_factory=list)
    links: CurseForgeLinks | None = None
    dateCreated: datetime | None = None
    dateModified: datetime | None = None
    dateReleased: datetime | None = None
    isAvailable: bool | None = None


class CurseForgeHash(CurseForgeModel):
    value: str
    algo: int


class CurseForgeDependency(CurseForgeModel):
    modId: int
    relationType: int


class CurseForgeModule(CurseForgeModel):
    name: str
    fingerprint: int


class CurseForgeFile(CurseForgeModel):
    id: int
    gameId: int
    modId: int
    isAvailable: bool
    displayName: str
    fileName: str
    releaseType: int
    fileStatus: int
    hashes: list[CurseForgeHash] = Field(default_factory=list)
    fileDate: datetime
    fileLength: int
    fileSizeOnDisk: int | None = None
    downloadUrl: str | None = None
    gameVersions: list[str] = Field(default_factory=list)
    dependencies: list[CurseForgeDependency] = Field(default_factory=list)
    fileFingerprint: int | None = None
    modules: list[CurseForgeModule] = Field(default_factory=list)


class CurseForgePagination(CurseForgeModel):
    index: int
    pageSize: int
    resultCount: int
    totalCount: int


class ModResponse(CurseForgeModel):
    data: CurseForgeMod


class FilesResponse(CurseForgeModel):
    data: list[CurseForgeFile]
    pagination: CurseForgePagination


class StringResponse(CurseForgeModel):
    data: str
