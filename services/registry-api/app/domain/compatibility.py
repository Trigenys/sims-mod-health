from __future__ import annotations

import uuid
from datetime import datetime

from sqlalchemy import CheckConstraint, DateTime, ForeignKey, Index, String, Text
from sqlalchemy.orm import Mapped, mapped_column

from app.db.base import Base


class GamePatch(Base):
    __tablename__ = "game_patches"

    id: Mapped[uuid.UUID] = mapped_column(primary_key=True, default=uuid.uuid4)
    normalized_version: Mapped[str] = mapped_column(
        String(96), unique=True, index=True
    )
    platform: Mapped[str] = mapped_column(String(32), default="windows")
    released_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True))
    source_id: Mapped[uuid.UUID] = mapped_column(
        ForeignKey("sources.id", ondelete="RESTRICT"), index=True
    )
    source_url: Mapped[str | None] = mapped_column(String(2048))
    retrieved_at: Mapped[datetime] = mapped_column(DateTime(timezone=True))


class CompatibilityReport(Base):
    __tablename__ = "compatibility_reports"
    __table_args__ = (
        Index(
            "ix_compatibility_release_patch_retrieved",
            "release_id",
            "patch_id",
            "retrieved_at",
        ),
        CheckConstraint(
            "("
            "(patch_id IS NOT NULL AND patch_min_version IS NULL AND patch_max_version IS NULL)"
            " OR "
            "(patch_id IS NULL AND (patch_min_version IS NOT NULL OR patch_max_version IS NOT NULL))"
            ")",
            name="ck_compatibility_patch_scope",
        ),
    )

    id: Mapped[uuid.UUID] = mapped_column(primary_key=True, default=uuid.uuid4)
    release_id: Mapped[uuid.UUID] = mapped_column(
        ForeignKey("mod_releases.id", ondelete="CASCADE"), index=True
    )
    patch_id: Mapped[uuid.UUID | None] = mapped_column(
        ForeignKey("game_patches.id", ondelete="CASCADE"), index=True
    )
    patch_min_version: Mapped[str | None] = mapped_column(String(96))
    patch_max_version: Mapped[str | None] = mapped_column(String(96))
    status: Mapped[str] = mapped_column(String(48))
    source_id: Mapped[uuid.UUID] = mapped_column(
        ForeignKey("sources.id", ondelete="RESTRICT"), index=True
    )
    source_url: Mapped[str | None] = mapped_column(String(2048))
    source_record_id: Mapped[str | None] = mapped_column(String(240))
    retrieved_at: Mapped[datetime] = mapped_column(DateTime(timezone=True))
    notes: Mapped[str | None] = mapped_column(Text)
