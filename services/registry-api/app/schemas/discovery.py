from __future__ import annotations

import uuid
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator


class StrictDiscoveryContract(BaseModel):
    model_config = ConfigDict(extra="forbid")


class DiscoveryRecommendRequest(StrictDiscoveryContract):
    patch_version: str = Field(min_length=1, max_length=96)
    platform: Literal["windows"] = "windows"
    installed_release_ids: list[uuid.UUID] = Field(min_length=1, max_length=500)
    limit: int = Field(default=12, ge=1, le=50)

    @model_validator(mode="after")
    def unique_release_ids(self) -> "DiscoveryRecommendRequest":
        if len(self.installed_release_ids) != len(set(self.installed_release_ids)):
            raise ValueError("installed_release_ids must be unique")
        return self


class RecommendationReason(StrictDiscoveryContract):
    because_mod_id: uuid.UUID
    because_mod_name: str
    shared_categories: list[str]
    shared_features: list[str]
    explanation: str


class DiscoveryRecommendation(StrictDiscoveryContract):
    mod_id: uuid.UUID
    release_id: uuid.UUID
    name: str
    creator_name: str
    categories: list[str]
    features: list[str]
    score: int
    reason: RecommendationReason


class DiscoveryRecommendResponse(StrictDiscoveryContract):
    patch_version: str
    platform: Literal["windows"]
    recommendations: list[DiscoveryRecommendation]
