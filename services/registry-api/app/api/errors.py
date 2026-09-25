from __future__ import annotations

from fastapi import Request
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse


async def validation_error_handler(
    _request: Request,
    error: RequestValidationError,
) -> JSONResponse:
    details = [
        {
            "field": ".".join(str(part) for part in item["loc"] if part != "body"),
            "message": item["msg"],
            "type": item["type"],
        }
        for item in error.errors()
    ]

    return JSONResponse(
        status_code=422,
        content={
            "error": {
                "code": "validation_error",
                "message": "Request validation failed.",
                "details": details,
            }
        },
    )
