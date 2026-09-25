from __future__ import annotations

import uuid
from dataclasses import dataclass

from sqlalchemy import select, tuple_
from sqlalchemy.orm import Session

from app.domain.catalog import Artifact, Mod, ModRelease
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


class ArtifactResolutionRepository:
    EXACT_KINDS = frozenset({"sha256", "curseforge"})

    def __init__(self, session: Session) -> None:
        self._session = session

    def find_exact(
        self,
        keys: set[FingerprintKey],
    ) -> dict[FingerprintKey, list[ArtifactResolutionRecord]]:
        exact_keys = {key for key in keys if key.kind in self.EXACT_KINDS}
        if not exact_keys:
            return {}

        identity_tuples = {
            (key.kind, key.value, key.algorithm_version) for key in exact_keys
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
