from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from app.db.session import get_session
from app.repositories.game_content_manifest import GameContentManifestRepository
from app.schemas.game_content import GameContentManifest
from app.services.game_content_manifest import (
    GameContentManifestService,
    GameContentManifestUnavailableError,
)


router = APIRouter(prefix="/game-content", tags=["game-content"])


@router.get("/manifest", response_model=GameContentManifest)
def latest_game_content_manifest(
    session: Session = Depends(get_session),
) -> GameContentManifest:
    service = GameContentManifestService(GameContentManifestRepository(session))
    try:
        return service.latest()
    except GameContentManifestUnavailableError as error:
        raise HTTPException(
            status_code=404,
            detail={
                "code": "game_content_manifest_unavailable",
                "message": str(error),
            },
        ) from error
