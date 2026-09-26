from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from app.db.session import get_session
from app.repositories.health_state import HealthStateRepository
from app.schemas.health import HealthEvaluateRequest, HealthEvaluateResponse
from app.services.health_state import HealthStateService, UnknownReleaseError


router = APIRouter(prefix="/health", tags=["health"])


@router.post("/evaluate", response_model=HealthEvaluateResponse)
def evaluate_health(
    request: HealthEvaluateRequest,
    session: Session = Depends(get_session),
) -> HealthEvaluateResponse:
    service = HealthStateService(HealthStateRepository(session))
    try:
        return service.evaluate(request)
    except UnknownReleaseError as error:
        raise HTTPException(
            status_code=404,
            detail={
                "code": "unknown_release",
                "message": "One or more installed releases are not present in the registry.",
                "release_ids": [str(item) for item in error.release_ids],
            },
        ) from error
