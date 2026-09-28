from __future__ import annotations

from sqlalchemy import select
from sqlalchemy.orm import Session

from app.domain.game_content import GameContentManifestDocument


class GameContentManifestRepository:
    def __init__(self, session: Session) -> None:
        self._session = session

    def latest(self) -> GameContentManifestDocument | None:
        statement = (
            select(GameContentManifestDocument)
            .order_by(
                GameContentManifestDocument.retrieved_at.desc(),
                GameContentManifestDocument.id.desc(),
            )
            .limit(1)
        )
        return self._session.scalar(statement)
