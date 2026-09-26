from __future__ import annotations

import uuid
from datetime import datetime
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator


class StrictRelationshipContract(BaseModel):
    model_config = ConfigDict(extra="forbid")


class InstallationRelationshipRequest(StrictRelationshipContract):
    installed_release_ids: list[uuid.UUID] = Field(min_length=1, max_length=500)

    @model_validator(mode="after")
    def unique_release_ids(self) -> "InstallationRelationshipRequest":
        if len(self.installed_release_ids) != len(set(self.installed_release_ids)):
            raise ValueError("installed_release_ids must be unique")
        return self


class RuleTarget(StrictRelationshipContract):
    mod_id: uuid.UUID | None = None
    source_kind: str | None = None
    source_external_id: str | None = None
    min_version: str | None = None
    max_version: str | None = None


class RuleProvenance(StrictRelationshipContract):
    source_id: uuid.UUID
    source_url: str | None = None
    source_record_id: str | None = None
    retrieved_at: datetime
    notes: str | None = None


class DependencyFinding(StrictRelationshipContract):
    rule_id: uuid.UUID
    required_by_release_id: uuid.UUID
    status: Literal["missing", "outdated", "version_mismatch"]
    target: RuleTarget
    installed_target_release_id: uuid.UUID | None = None
    installed_target_version: str | None = None
    action: Literal[
        "install_dependency",
        "update_dependency",
        "review_dependency_version",
    ]
    provenance: RuleProvenance


class KnownIncompatibilityFinding(StrictRelationshipContract):
    rule_id: uuid.UUID
    left_release_id: uuid.UUID
    right_release_id: uuid.UUID
    right_version: str | None = None
    target: RuleTarget
    provenance: RuleProvenance


class ReverseDependencyUsage(StrictRelationshipContract):
    dependency_release_id: uuid.UUID
    used_by_count: int = Field(ge=1)
    used_by_release_ids: list[uuid.UUID]


class DependencyCycle(StrictRelationshipContract):
    release_ids: list[uuid.UUID] = Field(min_length=1)


class InstallationRelationshipResponse(StrictRelationshipContract):
    dependency_findings: list[DependencyFinding]
    known_incompatibilities: list[KnownIncompatibilityFinding]
    reverse_usage: list[ReverseDependencyUsage]
    cycles: list[DependencyCycle]
