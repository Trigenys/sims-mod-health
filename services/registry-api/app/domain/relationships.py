from __future__ import annotations

import uuid
from datetime import datetime

from sqlalchemy import CheckConstraint, DateTime, ForeignKey, Index, String, Text
from sqlalchemy.orm import Mapped, mapped_column

from app.db.base import Base


class DependencyRule(Base):
    __tablename__ = "dependency_rules"
    __table_args__ = (
        CheckConstraint(
            "target_mod_id IS NOT NULL OR "
            "(target_source_kind IS NOT NULL AND target_source_external_id IS NOT NULL)",
            name="ck_dependency_rule_target",
        ),
        Index(
            "ix_dependency_rules_target_source",
            "target_source_kind",
            "target_source_external_id",
        ),
    )

    id: Mapped[uuid.UUID] = mapped_column(primary_key=True, default=uuid.uuid4)
    release_id: Mapped[uuid.UUID] = mapped_column(
        ForeignKey("mod_releases.id", ondelete="CASCADE"), index=True
    )
    target_mod_id: Mapped[uuid.UUID | None] = mapped_column(
        ForeignKey("mods.id", ondelete="CASCADE"), index=True
    )
    target_source_kind: Mapped[str | None] = mapped_column(String(48))
    target_source_external_id: Mapped[str | None] = mapped_column(String(240))
    min_version: Mapped[str | None] = mapped_column(String(160))
    max_version: Mapped[str | None] = mapped_column(String(160))
    source_id: Mapped[uuid.UUID] = mapped_column(
        ForeignKey("sources.id", ondelete="RESTRICT"), index=True
    )
    source_url: Mapped[str | None] = mapped_column(String(2048))
    source_record_id: Mapped[str | None] = mapped_column(String(240))
    retrieved_at: Mapped[datetime] = mapped_column(DateTime(timezone=True))
    notes: Mapped[str | None] = mapped_column(Text)


class ConflictRule(Base):
    __tablename__ = "conflict_rules"
    __table_args__ = (
        CheckConstraint(
            "target_mod_id IS NOT NULL OR "
            "(target_source_kind IS NOT NULL AND target_source_external_id IS NOT NULL)",
            name="ck_conflict_rule_target",
        ),
        Index(
            "ix_conflict_rules_target_source",
            "target_source_kind",
            "target_source_external_id",
        ),
    )

    id: Mapped[uuid.UUID] = mapped_column(primary_key=True, default=uuid.uuid4)
    release_id: Mapped[uuid.UUID] = mapped_column(
        ForeignKey("mod_releases.id", ondelete="CASCADE"), index=True
    )
    target_mod_id: Mapped[uuid.UUID | None] = mapped_column(
        ForeignKey("mods.id", ondelete="CASCADE"), index=True
    )
    target_source_kind: Mapped[str | None] = mapped_column(String(48))
    target_source_external_id: Mapped[str | None] = mapped_column(String(240))
    min_version: Mapped[str | None] = mapped_column(String(160))
    max_version: Mapped[str | None] = mapped_column(String(160))
    source_id: Mapped[uuid.UUID] = mapped_column(
        ForeignKey("sources.id", ondelete="RESTRICT"), index=True
    )
    source_url: Mapped[str | None] = mapped_column(String(2048))
    source_record_id: Mapped[str | None] = mapped_column(String(240))
    retrieved_at: Mapped[datetime] = mapped_column(DateTime(timezone=True))
    notes: Mapped[str | None] = mapped_column(Text)
