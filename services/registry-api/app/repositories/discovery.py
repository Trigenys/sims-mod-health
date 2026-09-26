from __future__ import annotations

import uuid
from dataclasses import dataclass

from sqlalchemy import select
from sqlalchemy.orm import Session

from app.domain import Creator, Mod, ModRelease


@dataclass(frozen=True, slots=True)
class DiscoveryModRecord:
    mod_id: uuid.UUID
    name: str
    creator_id: uuid.UUID
    creator_name: str
    categories: tuple[str, ...]
    features: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class InstalledDiscoveryRecord:
    release_id: uuid.UUID
    mod: DiscoveryModRecord


class DiscoveryRepository:
    def __init__(self, session: Session) -> None:
        self._session = session

    def get_installed(
        self,
        release_ids: set[uuid.UUID],
    ) -> dict[uuid.UUID, InstalledDiscoveryRecord]:
        if not release_ids:
            return {}

        statement = (
            select(
                ModRelease.id,
                Mod.id,
                Mod.name,
                Creator.id,
                Creator.display_name,
                Mod.categories,
                Mod.features,
            )
            .join(Mod, Mod.id == ModRelease.mod_id)
            .join(Creator, Creator.id == Mod.creator_id)
            .where(ModRelease.id.in_(release_ids))
        )

        return {
            row[0]: InstalledDiscoveryRecord(
                release_id=row[0],
                mod=DiscoveryModRecord(
                    mod_id=row[1],
                    name=row[2],
                    creator_id=row[3],
                    creator_name=row[4],
                    categories=tuple(row[5] or ()),
                    features=tuple(row[6] or ()),
                ),
            )
            for row in self._session.execute(statement)
        }

    def list_candidate_mods(
        self,
        excluded_mod_ids: set[uuid.UUID],
    ) -> list[DiscoveryModRecord]:
        statement = (
            select(
                Mod.id,
                Mod.name,
                Creator.id,
                Creator.display_name,
                Mod.categories,
                Mod.features,
            )
            .join(Creator, Creator.id == Mod.creator_id)
            .order_by(Mod.name, Mod.id)
        )
        if excluded_mod_ids:
            statement = statement.where(Mod.id.not_in(excluded_mod_ids))

        return [
            DiscoveryModRecord(
                mod_id=row[0],
                name=row[1],
                creator_id=row[2],
                creator_name=row[3],
                categories=tuple(row[4] or ()),
                features=tuple(row[5] or ()),
            )
            for row in self._session.execute(statement)
        ]
