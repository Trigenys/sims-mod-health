from __future__ import annotations

import uuid
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator


FingerprintKind = Literal[
    "sha256",
    "curseforge",
    "resource_signature",
    "script_signature",
]
ArtifactKind = Literal["package", "ts4script", "archive", "unknown"]


class StrictContract(BaseModel):
    model_config = ConfigDict(extra="forbid")


class FingerprintInput(StrictContract):
    kind: FingerprintKind
    value: str = Field(min_length=1, max_length=512)
    algorithm_version: str = Field(min_length=1, max_length=96)


class ArtifactProbe(StrictContract):
    client_ref: str = Field(min_length=1, max_length=128)
    artifact_kind: ArtifactKind | None = None
    filename: str | None = Field(default=None, max_length=512)
    size_bytes: int | None = Field(default=None, ge=0)
    fingerprints: list[FingerprintInput] = Field(min_length=1, max_length=8)

    @model_validator(mode="after")
    def unique_fingerprints(self) -> "ArtifactProbe":
        identities = [
            (item.kind, item.value, item.algorithm_version)
            for item in self.fingerprints
        ]
        if len(identities) != len(set(identities)):
            raise ValueError("fingerprints must be unique within one artifact probe")
        return self


class BatchResolveRequest(StrictContract):
    artifacts: list[ArtifactProbe] = Field(min_length=1, max_length=100)

    @model_validator(mode="after")
    def unique_client_refs(self) -> "BatchResolveRequest":
        refs = [artifact.client_ref for artifact in self.artifacts]
        if len(refs) != len(set(refs)):
            raise ValueError("client_ref must be unique within one batch")
        return self


class MatchEvidence(StrictContract):
    type: Literal["exact_fingerprint"] = "exact_fingerprint"
    kind: Literal["sha256", "curseforge"]
    value: str
    algorithm_version: str


class ArtifactMatch(StrictContract):
    artifact_id: uuid.UUID
    release_id: uuid.UUID
    mod_id: uuid.UUID
    creator_id: uuid.UUID
    confidence: Literal["exact"] = "exact"
    evidence: list[MatchEvidence]


class ArtifactResolution(StrictContract):
    client_ref: str
    matches: list[ArtifactMatch]


class BatchResolveResponse(StrictContract):
    artifacts: list[ArtifactResolution]
