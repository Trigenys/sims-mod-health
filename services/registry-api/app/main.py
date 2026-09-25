from __future__ import annotations

from fastapi import FastAPI
from fastapi.exceptions import RequestValidationError

from app.api.errors import validation_error_handler
from app.api.v1.router import router as v1_router
from app.config import get_settings


settings = get_settings()
app = FastAPI(title=settings.api_title, version=settings.api_version)
app.add_exception_handler(RequestValidationError, validation_error_handler)
app.include_router(v1_router)


@app.get("/health", tags=["system"])
def health() -> dict[str, str]:
    return {"status": "ok"}
