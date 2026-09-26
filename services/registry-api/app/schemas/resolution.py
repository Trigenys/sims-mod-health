from __future__ import annotations

import uuid
from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator


FingerprintKind = Literal[
    "sha256",
    "curseforge",
    "resource_signature",
    "script_signature",
]
ArtifactKind = Literal["package", "ts4script", "archive", "unknown"]
ResolutionStatus = Literal["resolved", "ambiguous", "unresolved"]
MatchConfidence = Literal["exact", "high", "medium", "low"]
MatchStage = Literal[
    "exact_fingerprint",
    "embedded_metadata",
    "normalized_name",
    "resource_signature",
    "fuzzy_candidate",
]


class StrictContract(BaseModel):
    model_config = ConfigDict(extra="forbid")


class FingerprintInput(StrictContract):
    kind: FingerprintKind
    value: str = Field(min_length=1, max_length=512)
    algorithm_version: str = Field(min_length=1, max_length=96)


class IdentityHints(StrictContract):
    creator: str | None = Field(default=None, min_length=1, max_length=240)
    mod_name: str | None = Field(default=None, min_length=1, max_length=280)
    version: str | None = Field(default=None, min_length=1, max_length=160)


class ArtifactProbe(StrictContract):
    client_ref: str = Field(min_length=1, max_length=128)
    artifact_kind: ArtifactKind | None = None
    filename: str | None = Field(default=None, max_length=512)
    size_bytes: int | None = Field(default=None, ge=0)
    identity_hints: IdentityHints | None = None
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


class ExactFingerprintEvidence(StrictContract):
    type: Literal["exact_fingerprint"] = "exact_fingerprint"
    kind: Literal["sha256", "curseforge"]
    value: str
    algorithm_version: str


class EmbeddedMetadataEvidence(StrictContract):
    type: Literal["embedded_metadata"] = "embedded_metadata"
    field: Literal["creator", "mod_name", "version"]
    value: str


class NormalizedFilenameEvidence(StrictContract):
    type: Literal["normalized_filename"] = "normalized_filename"
    local_filename: str
    registry_filename: str


class StructuralFingerprintEvidence(StrictContract):
    type: Literal["structural_fingerprint"] = "structural_fingerprint"
    kind: Literal["resource_signature", "script_signature"]
    value: str
    algorithm_version: str


class FuzzyNameEvidence(StrictContract):
    type: Literal["fuzzy_name"] = "fuzzy_name"
    local_value: str
    registry_value: str
    similarity: float = Field(ge=0.0, le=1.0)


class SizeEvidence(StrictContract):
    type: Literal["size"] = "size"
    size_bytes: int = Field(ge=0)


MatchEvidence = Annotated[
    ExactFingerprintEvidence
    | EmbeddedMetadataEvidence
    | NormalizedFilenameEvidence
    | StructuralFingerprintEvidence
    | FuzzyNameEvidence
    | SizeEvidence,
    Field(discriminator="type"),
]


class ArtifactMatch(StrictContract):
    artifact_id: uuid.UUID
    release_id: uuid.UUID
    mod_id: uuid.UUID
    creator_id: uuid.UUID
    stage: MatchStage
    confidence: MatchConfidence
    score: float = Field(ge=0.0, le=1.0)
    deterministic: bool
    evidence: list[MatchEvidence]


class ArtifactResolution(StrictContract):
    client_ref: str
    status: ResolutionStatus
    selected_artifact_id: uuid.UUID | None = None
    matches: list[ArtifactMatch]


class BatchResolveResponse(StrictContract):
    artifacts: list[ArtifactResolution]
