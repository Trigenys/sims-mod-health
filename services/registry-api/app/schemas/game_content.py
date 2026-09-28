from __future__ import annotations

from datetime import datetime
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator


class StrictGameContentContract(BaseModel):
    model_config = ConfigDict(extra="forbid")


class ManifestFingerprint(StrictGameContentContract):
    relative_path: str = Field(min_length=1, max_length=512)
    sha256: str = Field(pattern=r"^[0-9a-fA-F]{64}$")


class GameBuildManifestEntry(StrictGameContentContract):
    version: str = Field(min_length=1, max_length=96)
    released_at: datetime | None = None
    fingerprints: list[ManifestFingerprint] = Field(default_factory=list, max_length=32)


class PackManifestEntry(StrictGameContentContract):
    code: str = Field(pattern=r"^(EP|GP|SP|FP)[0-9]{2,3}$|^KIT[0-9]{2,3}$")
    pack_kind: Literal["expansion", "game", "stuff_or_kit", "free", "kit", "unknown"]
    min_game_version: str | None = Field(default=None, max_length=96)
    released_at: datetime | None = None
    expected_fingerprints: list[ManifestFingerprint] = Field(default_factory=list, max_length=32)
    evidence_source: str = Field(min_length=1, max_length=240)
    evidence_url: str | None = Field(default=None, max_length=2048)

    @field_validator("code")
    @classmethod
    def normalize_code(cls, value: str) -> str:
        return value.upper()


class GameContentManifest(StrictGameContentContract):
    schema_version: Literal[1] = 1
    manifest_version: str = Field(min_length=1, max_length=96)
    source_identity: str = Field(min_length=1, max_length=240)
    source_url: str | None = Field(default=None, max_length=2048)
    retrieved_at: datetime
    expires_at: datetime | None = None
    checksum_sha256: str | None = Field(
        default=None,
        pattern=r"^[0-9a-fA-F]{64}$",
    )
    signature: str | None = Field(default=None, max_length=4096)
    latest_game_build: str = Field(min_length=1, max_length=96)
    game_builds: list[GameBuildManifestEntry] = Field(default_factory=list, max_length=512)
    packs: list[PackManifestEntry] = Field(default_factory=list, max_length=512)

    @model_validator(mode="after")
    def latest_build_is_known(self) -> "GameContentManifest":
        known = {entry.version for entry in self.game_builds}
        if self.latest_game_build not in known:
            raise ValueError("latest_game_build must exist in game_builds")
        return self
