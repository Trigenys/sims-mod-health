from __future__ import annotations

import json

from fastapi.testclient import TestClient

from app.main import app
from app.schemas.resolution import BatchResolveRequest


client = TestClient(app)


def test_health_endpoint() -> None:
    response = client.get("/health")

    assert response.status_code == 200
    assert response.json() == {"status": "ok"}


def test_validation_error_is_stable_and_actionable() -> None:
    response = client.post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "local-1",
                    "fingerprints": [],
                }
            ]
        },
    )

    assert response.status_code == 422
    payload = response.json()
    assert payload["error"]["code"] == "validation_error"
    assert payload["error"]["message"] == "Request validation failed."
    assert payload["error"]["details"]
    assert payload["error"]["details"][0]["field"] == "artifacts.0.fingerprints"


def test_contract_rejects_raw_file_payloads() -> None:
    response = client.post(
        "/v1/artifacts/resolve",
        json={
            "artifacts": [
                {
                    "client_ref": "local-1",
                    "fingerprints": [
                        {
                            "kind": "sha256",
                            "value": "a" * 64,
                            "algorithm_version": "sha256-v1",
                        }
                    ],
                    "raw_file_base64": "not-allowed",
                }
            ]
        },
    )

    assert response.status_code == 422
    details = response.json()["error"]["details"]
    assert any(
        item["field"] == "artifacts.0.raw_file_base64"
        and item["type"] == "extra_forbidden"
        for item in details
    )


def test_resolution_schema_contains_no_raw_file_contract() -> None:
    schema = json.dumps(BatchResolveRequest.model_json_schema()).lower()

    assert "raw_file" not in schema
    assert "file_bytes" not in schema
    assert "base64" not in schema
