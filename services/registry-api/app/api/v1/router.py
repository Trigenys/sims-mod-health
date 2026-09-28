from fastapi import APIRouter

from app.api.v1.routes.artifacts import router as artifacts_router
from app.api.v1.routes.discovery import router as discovery_router
from app.api.v1.routes.health import router as health_router
from app.api.v1.routes.game_content import router as game_content_router
from app.api.v1.routes.relationships import router as relationships_router


router = APIRouter(prefix="/v1")
router.include_router(artifacts_router)
router.include_router(discovery_router)
router.include_router(health_router)
router.include_router(game_content_router)
router.include_router(relationships_router)
