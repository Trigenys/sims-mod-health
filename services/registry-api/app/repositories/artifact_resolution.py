from __future__ import annotations

import uuid
from dataclasses import dataclass

from sqlalchemy import select, tuple_
from sqlalchemy.orm import Session

from app.domain.catalog import Artifact, Creator, Mod, ModRelease
from app.domain.identity import Fingerprint


@dataclass(frozen=True, slots=True)
class FingerprintKey:
    kind: str
    value: str
    algorithm_version: str


@dataclass(frozen=True, slots=True)
class ArtifactResolutionRecord:
    artifact_id: uuid.UUID
    release_id: uuid.UUID
    mod_id: uuid.UUID
    creator_id: uuid.UUID
    fingerprint: FingerprintKey


@dataclass(frozen=True, slots=True)
class ArtifactCandidateRecord:
    artifact_id: uuid.UUID
    release_id: uuid.UUID
    mod_id: uuid.UUID
    creator_id: uuid.UUID
    artifact_kind: str
    filename: str | None
    size_bytes: int | None
    release_version: str | None
    mod_name: str
    mod_slug: str
    mod_aliases: tuple[str, ...]
    creator_name: str
    creator_slug: str
    creator_aliases: tuple[str, ...]


class ArtifactResolutionRepository:
    EXACT_KINDS = frozenset({"sha256", "curseforge"})
    STRUCTURAL_KINDS = frozenset({"resource_signature", "script_signature"})
    MAX_CANDIDATES_PER_KIND = 1_000

    def __init__(self, session: Session) -> None:
        self._session = session

    def find_exact(
        self,
        keys: set[FingerprintKey],
    ) -> dict[FingerprintKey, list[ArtifactResolutionRecord]]:
        return self._find_fingerprints(keys, self.EXACT_KINDS)

    def find_structural(
        self,
        keys: set[FingerprintKey],
    ) -> dict[FingerprintKey, list[ArtifactResolutionRecord]]:
        return self._find_fingerprints(keys, self.STRUCTURAL_KINDS)

    def list_candidates(
        self,
        artifact_kind: str | None,
    ) -> list[ArtifactCandidateRecord]:
        statement = (
            select(
                Artifact.id,
                ModRelease.id,
                Mod.id,
                Mod.creator_id,
                Artifact.artifact_kind,
                Artifact.filename,
                Artifact.size_bytes,
                ModRelease.version,
                Mod.name,
                Mod.slug,
                Mod.aliases,
                Creator.display_name,
                Creator.slug,
                Creator.aliases,
            )
            .join(ModRelease, ModRelease.id == Artifact.release_id)
            .join(Mod, Mod.id == ModRelease.mod_id)
            .join(Creator, Creator.id == Mod.creator_id)
            .order_by(Artifact.id)
            .limit(self.MAX_CANDIDATES_PER_KIND)
        )

        if artifact_kind and artifact_kind != "unknown":
            statement = statement.where(Artifact.artifact_kind == artifact_kind)

        return [
            ArtifactCandidateRecord(
                artifact_id=row[0],
                release_id=row[1],
                mod_id=row[2],
                creator_id=row[3],
                artifact_kind=row[4],
                filename=row[5],
                size_bytes=row[6],
                release_version=row[7],
                mod_name=row[8],
                mod_slug=row[9],
                mod_aliases=tuple(row[10] or []),
                creator_name=row[11],
                creator_slug=row[12],
                creator_aliases=tuple(row[13] or []),
            )
            for row in self._session.execute(statement)
        ]

    def _find_fingerprints(
        self,
        keys: set[FingerprintKey],
        allowed_kinds: frozenset[str],
    ) -> dict[FingerprintKey, list[ArtifactResolutionRecord]]:
        selected_keys = {key for key in keys if key.kind in allowed_kinds}
        if not selected_keys:
            return {}

        identity_tuples = {
            (key.kind, key.value, key.algorithm_version) for key in selected_keys
        }

        statement = (
            select(
                Fingerprint.kind,
                Fingerprint.value,
                Fingerprint.algorithm_version,
                Artifact.id,
                ModRelease.id,
                Mod.id,
                Mod.creator_id,
            )
            .join(Artifact, Artifact.id == Fingerprint.artifact_id)
            .join(ModRelease, ModRelease.id == Artifact.release_id)
            .join(Mod, Mod.id == ModRelease.mod_id)
            .where(
                tuple_(
                    Fingerprint.kind,
                    Fingerprint.value,
                    Fingerprint.algorithm_version,
                ).in_(identity_tuples)
            )
            .order_by(
                Fingerprint.kind,
                Fingerprint.value,
                Fingerprint.algorithm_version,
                Artifact.id,
            )
        )

        grouped: dict[FingerprintKey, list[ArtifactResolutionRecord]] = {}
        for row in self._session.execute(statement):
            key = FingerprintKey(row[0], row[1], row[2])
            grouped.setdefault(key, []).append(
                ArtifactResolutionRecord(
                    artifact_id=row[3],
                    release_id=row[4],
                    mod_id=row[5],
                    creator_id=row[6],
                    fingerprint=key,
                )
            )

        return grouped
