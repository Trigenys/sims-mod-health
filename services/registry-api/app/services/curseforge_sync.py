from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Literal

from sqlalchemy.orm import Session

from app.repositories.curseforge_ingestion import CurseForgeIngestionRepository
from app.sources.curseforge.client import (
    CurseForgeClient,
    CurseForgeNotFound,
    CurseForgeUnavailable,
)
from app.sources.curseforge.mapper import CurseForgeMapper


@dataclass(frozen=True, slots=True)
class CurseForgeSyncResult:
    status: Literal["updated", "not_found", "unavailable"]
    mod_id: int
    releases_upserted: int = 0
    artifacts_upserted: int = 0
    retrieved_at: datetime | None = None
    retry_after_seconds: float | None = None


class CurseForgeSyncService:
    def __init__(
        self,
        session: Session,
        client: CurseForgeClient,
        *,
        mapper: CurseForgeMapper | None = None,
    ) -> None:
        self._session = session
        self._client = client
        self._mapper = mapper or CurseForgeMapper()
        self._repository = CurseForgeIngestionRepository(session)

    def sync_mod(
        self,
        mod_id: int,
        *,
        include_changelogs: bool = False,
    ) -> CurseForgeSyncResult:
        retrieved_at = datetime.now(timezone.utc)

        try:
            mod = self._client.get_mod(mod_id)
            files = self._client.get_mod_files(mod_id)
            changelogs: dict[int, str] = {}
            if include_changelogs:
                for file in files:
                    try:
                        changelogs[file.id] = self._client.get_file_changelog(
                            mod_id,
                            file.id,
                        )
                    except CurseForgeUnavailable:
                        # Changelog enrichment is optional. Do not fail the canonical file sync.
                        continue
        except CurseForgeNotFound:
            self._session.rollback()
            return CurseForgeSyncResult(status="not_found", mod_id=mod_id)
        except CurseForgeUnavailable as error:
            self._session.rollback()
            return CurseForgeSyncResult(
                status="unavailable",
                mod_id=mod_id,
                retry_after_seconds=error.retry_after_seconds,
            )

        project = self._mapper.map_project(
            mod,
            files,
            retrieved_at=retrieved_at,
            changelogs=changelogs,
        )

        try:
            releases, artifacts = self._repository.upsert_project(
                project,
                retrieved_at=retrieved_at,
            )
            self._session.commit()
        except Exception:
            self._session.rollback()
            raise

        return CurseForgeSyncResult(
            status="updated",
            mod_id=mod_id,
            releases_upserted=releases,
            artifacts_upserted=artifacts,
            retrieved_at=retrieved_at,
        )
