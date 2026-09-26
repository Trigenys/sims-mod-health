from __future__ import annotations

import uuid
from dataclasses import dataclass
from datetime import datetime

from sqlalchemy import select
from sqlalchemy.orm import Session

from app.domain import ConflictRule, DependencyRule, ModRelease, Source


@dataclass(frozen=True, slots=True)
class InstalledReleaseRecord:
    release_id: uuid.UUID
    mod_id: uuid.UUID
    version: str | None
    released_at: datetime | None
    source_kind: str | None
    source_external_id: str | None


@dataclass(frozen=True, slots=True)
class RelationshipRuleRecord:
    rule_id: uuid.UUID
    release_id: uuid.UUID
    target_mod_id: uuid.UUID | None
    target_source_kind: str | None
    target_source_external_id: str | None
    min_version: str | None
    max_version: str | None
    source_id: uuid.UUID
    source_url: str | None
    source_record_id: str | None
    retrieved_at: datetime
    notes: str | None


class RelationshipAnalysisRepository:
    def __init__(self, session: Session) -> None:
        self._session = session

    def get_installed_releases(
        self,
        release_ids: set[uuid.UUID],
    ) -> dict[uuid.UUID, InstalledReleaseRecord]:
        if not release_ids:
            return {}

        statement = (
            select(
                ModRelease.id,
                ModRelease.mod_id,
                ModRelease.version,
                ModRelease.released_at,
                Source.kind,
                Source.external_id,
            )
            .outerjoin(Source, Source.id == ModRelease.source_id)
            .where(ModRelease.id.in_(release_ids))
        )

        return {
            row[0]: InstalledReleaseRecord(
                release_id=row[0],
                mod_id=row[1],
                version=row[2],
                released_at=row[3],
                source_kind=row[4],
                source_external_id=row[5],
            )
            for row in self._session.execute(statement)
        }

    def list_dependency_rules(
        self,
        release_ids: set[uuid.UUID],
    ) -> list[RelationshipRuleRecord]:
        return self._list_rules(DependencyRule, release_ids)

    def list_conflict_rules(
        self,
        release_ids: set[uuid.UUID],
    ) -> list[RelationshipRuleRecord]:
        return self._list_rules(ConflictRule, release_ids)

    def _list_rules(
        self,
        model: type[DependencyRule] | type[ConflictRule],
        release_ids: set[uuid.UUID],
    ) -> list[RelationshipRuleRecord]:
        if not release_ids:
            return []

        statement = (
            select(
                model.id,
                model.release_id,
                model.target_mod_id,
                model.target_source_kind,
                model.target_source_external_id,
                model.min_version,
                model.max_version,
                model.source_id,
                model.source_url,
                model.source_record_id,
                model.retrieved_at,
                model.notes,
            )
            .where(model.release_id.in_(release_ids))
            .order_by(model.release_id, model.id)
        )

        return [
            RelationshipRuleRecord(
                rule_id=row[0],
                release_id=row[1],
                target_mod_id=row[2],
                target_source_kind=row[3],
                target_source_external_id=row[4],
                min_version=row[5],
                max_version=row[6],
                source_id=row[7],
                source_url=row[8],
                source_record_id=row[9],
                retrieved_at=row[10],
                notes=row[11],
            )
            for row in self._session.execute(statement)
        ]
