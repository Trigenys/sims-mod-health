from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from app.db.session import get_session
from app.repositories.relationship_analysis import RelationshipAnalysisRepository
from app.schemas.relationships import (
    InstallationRelationshipRequest,
    InstallationRelationshipResponse,
)
from app.services.relationship_analysis import (
    RelationshipAnalysisService,
    UnknownInstalledReleaseError,
)


router = APIRouter(prefix="/health/relationships", tags=["health"])


@router.post("/evaluate", response_model=InstallationRelationshipResponse)
def evaluate_relationships(
    request: InstallationRelationshipRequest,
    session: Session = Depends(get_session),
) -> InstallationRelationshipResponse:
    service = RelationshipAnalysisService(
        RelationshipAnalysisRepository(session)
    )
    try:
        return service.analyze(request)
    except UnknownInstalledReleaseError as error:
        raise HTTPException(
            status_code=404,
            detail={
                "code": "unknown_release",
                "message": "One or more installed releases are not present in the registry.",
                "release_ids": [str(item) for item in error.release_ids],
            },
        ) from error
