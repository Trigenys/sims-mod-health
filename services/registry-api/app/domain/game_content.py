from __future__ import annotations

import uuid
from datetime import datetime
from typing import Any

from sqlalchemy import DateTime, Integer, JSON, String, UniqueConstraint
from sqlalchemy.orm import Mapped, mapped_column

from app.db.base import Base


class GameContentManifestDocument(Base):
    __tablename__ = "game_content_manifests"
    __table_args__ = (
        UniqueConstraint(
            "source_identity",
            "manifest_version",
            name="uq_game_content_manifest_source_version",
        ),
    )

    id: Mapped[uuid.UUID] = mapped_column(primary_key=True, default=uuid.uuid4)
    schema_version: Mapped[int] = mapped_column(Integer)
    manifest_version: Mapped[str] = mapped_column(String(96))
    source_identity: Mapped[str] = mapped_column(String(240), index=True)
    source_url: Mapped[str | None] = mapped_column(String(2048))
    retrieved_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), index=True)
    expires_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True))
    checksum_sha256: Mapped[str | None] = mapped_column(String(64))
    signature: Mapped[str | None] = mapped_column(String(4096))
    payload_json: Mapped[dict[str, Any]] = mapped_column(JSON)
