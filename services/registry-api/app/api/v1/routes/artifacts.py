from __future__ import annotations

from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.db.session import get_session
from app.repositories.artifact_resolution import ArtifactResolutionRepository
from app.schemas.resolution import BatchResolveRequest, BatchResolveResponse
from app.services.artifact_resolution import ArtifactResolutionService


router = APIRouter(prefix="/artifacts", tags=["artifacts"])


@router.post("/resolve", response_model=BatchResolveResponse)
def resolve_artifacts(
    request: BatchResolveRequest,
    session: Session = Depends(get_session),
) -> BatchResolveResponse:
    repository = ArtifactResolutionRepository(session)
    return ArtifactResolutionService(repository).resolve(request)
