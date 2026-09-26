from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from app.db.session import get_session
from app.repositories.discovery import DiscoveryRepository
from app.repositories.health_state import HealthStateRepository
from app.repositories.relationship_analysis import RelationshipAnalysisRepository
from app.schemas.discovery import (
    DiscoveryRecommendRequest,
    DiscoveryRecommendResponse,
)
from app.services.discovery import (
    DiscoveryRecommendationService,
    UnknownDiscoveryReleaseError,
)


router = APIRouter(prefix="/discovery", tags=["discovery"])


@router.post("/recommend", response_model=DiscoveryRecommendResponse)
def recommend_mods(
    request: DiscoveryRecommendRequest,
    session: Session = Depends(get_session),
) -> DiscoveryRecommendResponse:
    service = DiscoveryRecommendationService(
        DiscoveryRepository(session),
        HealthStateRepository(session),
        RelationshipAnalysisRepository(session),
    )
    try:
        return service.recommend(request)
    except UnknownDiscoveryReleaseError as error:
        raise HTTPException(
            status_code=404,
            detail={
                "code": "unknown_release",
                "message": "One or more installed releases are not present in the registry.",
                "release_ids": [str(item) for item in error.release_ids],
            },
        ) from error
