from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any

from app.sources.curseforge.models import CurseForgeFile, CurseForgeMod


RELATION_NAMES = {
    1: "embedded_library",
    2: "optional_dependency",
    3: "required_dependency",
    4: "tool",
    5: "incompatible",
    6: "include",
}
HASH_NAMES = {1: "sha1", 2: "md5"}
RELEASE_NAMES = {1: "release", 2: "beta", 3: "alpha"}


@dataclass(frozen=True, slots=True)
class MappedFingerprint:
    kind: str
    value: str
    algorithm_version: str


@dataclass(frozen=True, slots=True)
class MappedFile:
    source_record_id: str
    display_name: str
    filename: str
    artifact_kind: str
    size_bytes: int
    source_url: str | None
    released_at: datetime
    changelog: str | None
    fingerprints: tuple[MappedFingerprint, ...]
    metadata: dict[str, Any]


@dataclass(frozen=True, slots=True)
class MappedProject:
    source_external_id: str
    source_url: str | None
    game_id: int
    creator_slug: str
    creator_name: str
    creator_aliases: tuple[str, ...]
    mod_slug: str
    mod_name: str
    summary: str | None
    source_metadata: dict[str, Any]
    files: tuple[MappedFile, ...]


class CurseForgeMapper:
    def map_project(
        self,
        mod: CurseForgeMod,
        files: list[CurseForgeFile],
        *,
        retrieved_at: datetime,
        changelogs: dict[int, str] | None = None,
    ) -> MappedProject:
        authors = mod.authors
        primary = authors[0] if authors else None
        creator_slug = (
            f"curseforge-author-{primary.id}"
            if primary is not None
            else f"curseforge-project-{mod.id}"
        )
        creator_name = primary.name if primary is not None else mod.name
        creator_aliases = tuple(
            dict.fromkeys(author.name for author in authors if author.name != creator_name)
        )
        website_url = mod.links.websiteUrl if mod.links else None

        mapped_files = tuple(
            self.map_file(
                file,
                changelog=(changelogs or {}).get(file.id),
            )
            for file in sorted(files, key=lambda item: (item.fileDate, item.id))
        )

        return MappedProject(
            source_external_id=str(mod.id),
            source_url=website_url,
            game_id=mod.gameId,
            creator_slug=creator_slug,
            creator_name=creator_name,
            creator_aliases=creator_aliases,
            mod_slug=mod.slug,
            mod_name=mod.name,
            summary=mod.summary,
            source_metadata={
                "game_id": mod.gameId,
                "project_id": mod.id,
                "slug": mod.slug,
                "status": mod.status,
                "is_available": mod.isAvailable,
                "authors": [author.model_dump(mode="json") for author in authors],
                "links": mod.links.model_dump(mode="json") if mod.links else None,
                "date_created": _iso(mod.dateCreated),
                "date_modified": _iso(mod.dateModified),
                "date_released": _iso(mod.dateReleased),
                "retrieved_at": retrieved_at.isoformat(),
            },
            files=mapped_files,
        )

    def map_file(
        self,
        file: CurseForgeFile,
        *,
        changelog: str | None = None,
    ) -> MappedFile:
        fingerprints: list[MappedFingerprint] = []
        if file.fileFingerprint is not None:
            fingerprints.append(
                MappedFingerprint(
                    kind="curseforge",
                    value=str(file.fileFingerprint),
                    algorithm_version="curseforge-file-fingerprint-v1",
                )
            )

        for file_hash in file.hashes:
            kind = HASH_NAMES.get(file_hash.algo)
            if kind is None:
                continue
            fingerprints.append(
                MappedFingerprint(
                    kind=kind,
                    value=file_hash.value.lower(),
                    algorithm_version=f"{kind}-curseforge-v1",
                )
            )

        dependencies = [
            {
                "mod_id": dependency.modId,
                "relation_type": dependency.relationType,
                "relation": RELATION_NAMES.get(
                    dependency.relationType,
                    f"unknown_{dependency.relationType}",
                ),
            }
            for dependency in file.dependencies
        ]

        return MappedFile(
            source_record_id=str(file.id),
            display_name=file.displayName,
            filename=file.fileName,
            artifact_kind=_artifact_kind(file.fileName),
            size_bytes=file.fileLength,
            source_url=file.downloadUrl,
            released_at=file.fileDate,
            changelog=changelog,
            fingerprints=tuple(fingerprints),
            metadata={
                "curseforge": {
                    "file_id": file.id,
                    "mod_id": file.modId,
                    "game_id": file.gameId,
                    "is_available": file.isAvailable,
                    "release_type": file.releaseType,
                    "release_type_name": RELEASE_NAMES.get(file.releaseType),
                    "file_status": file.fileStatus,
                    "file_size_on_disk": file.fileSizeOnDisk,
                    "game_versions": list(file.gameVersions),
                    "dependencies": dependencies,
                    "hashes": [item.model_dump(mode="json") for item in file.hashes],
                    "modules": [item.model_dump(mode="json") for item in file.modules],
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


def _iso(value: datetime | None) -> str | None:
    return value.isoformat() if value is not None else None
