from __future__ import annotations

import uuid
from dataclasses import dataclass
from datetime import datetime

from sqlalchemy import select
from sqlalchemy.orm import Session

from app.domain import CompatibilityReport, GamePatch, ModRelease


@dataclass(frozen=True, slots=True)
class ReleaseRecord:
    id: uuid.UUID
    mod_id: uuid.UUID
    version: str | None
    released_at: datetime | None


@dataclass(frozen=True, slots=True)
class CompatibilityEvidenceRecord:
    report_id: uuid.UUID
    release_id: uuid.UUID
    status: str
    source_id: uuid.UUID
    source_url: str | None
    source_record_id: str | None
    retrieved_at: datetime
    notes: str | None
    patch_id: uuid.UUID | None
    patch_version: str | None
    patch_min_version: str | None
    patch_max_version: str | None


class HealthStateRepository:
    def __init__(self, session: Session) -> None:
        self._session = session

    def get_releases(self, release_ids: set[uuid.UUID]) -> dict[uuid.UUID, ReleaseRecord]:
        if not release_ids:
            return {}

        statement = select(
            ModRelease.id,
            ModRelease.mod_id,
            ModRelease.version,
            ModRelease.released_at,
        ).where(ModRelease.id.in_(release_ids))

        return {
            row[0]: ReleaseRecord(
                id=row[0],
                mod_id=row[1],
                version=row[2],
                released_at=row[3],
            )
            for row in self._session.execute(statement)
        }

    def list_releases_for_mods(self, mod_ids: set[uuid.UUID]) -> list[ReleaseRecord]:
        if not mod_ids:
            return []

        statement = (
            select(
                ModRelease.id,
                ModRelease.mod_id,
                ModRelease.version,
                ModRelease.released_at,
            )
            .where(ModRelease.mod_id.in_(mod_ids))
            .order_by(ModRelease.mod_id, ModRelease.released_at, ModRelease.id)
        )

        return [
            ReleaseRecord(
                id=row[0],
                mod_id=row[1],
                version=row[2],
                released_at=row[3],
            )
            for row in self._session.execute(statement)
        ]

    def list_compatibility_evidence(
        self,
        release_ids: set[uuid.UUID],
    ) -> list[CompatibilityEvidenceRecord]:
        if not release_ids:
            return []

        statement = (
            select(
                CompatibilityReport.id,
                CompatibilityReport.release_id,
                CompatibilityReport.status,
                CompatibilityReport.source_id,
                CompatibilityReport.source_url,
                CompatibilityReport.source_record_id,
                CompatibilityReport.retrieved_at,
                CompatibilityReport.notes,
                CompatibilityReport.patch_id,
                GamePatch.normalized_version,
                CompatibilityReport.patch_min_version,
                CompatibilityReport.patch_max_version,
            )
            .outerjoin(GamePatch, GamePatch.id == CompatibilityReport.patch_id)
            .where(CompatibilityReport.release_id.in_(release_ids))
            .order_by(
                CompatibilityReport.release_id,
                CompatibilityReport.retrieved_at.desc(),
                CompatibilityReport.id,
            )
        )

        return [
            CompatibilityEvidenceRecord(
                report_id=row[0],
                release_id=row[1],
                status=row[2],
                source_id=row[3],
                source_url=row[4],
                source_record_id=row[5],
                retrieved_at=row[6],
                notes=row[7],
                patch_id=row[8],
                patch_version=row[9],
                patch_min_version=row[10],
                patch_max_version=row[11],
            )
            for row in self._session.execute(statement)
        ]
