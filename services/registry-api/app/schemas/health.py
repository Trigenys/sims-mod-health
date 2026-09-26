from __future__ import annotations

import uuid
from datetime import datetime
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator


HealthState = Literal[
    "compatible",
    "update_available",
    "unknown",
    "potential_conflict",
    "broken",
    "abandoned",
]
CompatibilityState = Literal[
    "compatible",
    "unknown",
    "potential_conflict",
    "broken",
    "abandoned",
]


class StrictHealthContract(BaseModel):
    model_config = ConfigDict(extra="forbid")


class HealthEvaluateRequest(StrictHealthContract):
    patch_version: str = Field(min_length=1, max_length=96)
    platform: Literal["windows"] = "windows"
    installed_release_ids: list[uuid.UUID] = Field(min_length=1, max_length=100)

    @model_validator(mode="after")
    def unique_release_ids(self) -> "HealthEvaluateRequest":
        if len(self.installed_release_ids) != len(set(self.installed_release_ids)):
            raise ValueError("installed_release_ids must be unique")
        return self


class CompatibilityEvidence(StrictHealthContract):
    report_id: uuid.UUID
    source_id: uuid.UUID
    source_url: str | None = None
    source_record_id: str | None = None
    status: str
    retrieved_at: datetime
    notes: str | None = None
    scope: Literal["exact_patch", "patch_range"]
    patch_version: str | None = None
    patch_min_version: str | None = None
    patch_max_version: str | None = None


class UpdateAssessment(StrictHealthContract):
    available: bool
    target_release_id: uuid.UUID | None = None
    target_version: str | None = None
    target_compatibility_state: CompatibilityState | None = None
    target_disputed: bool = False


class HealthAssessment(StrictHealthContract):
    release_id: uuid.UUID
    mod_id: uuid.UUID
    state: HealthState
    compatibility_state: CompatibilityState
    disputed: bool
    reason: str
    evidence: list[CompatibilityEvidence]
    update: UpdateAssessment


class HealthEvaluateResponse(StrictHealthContract):
    patch_version: str
    platform: Literal["windows"]
    items: list[HealthAssessment]
