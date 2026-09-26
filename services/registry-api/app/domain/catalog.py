from __future__ import annotations

import uuid
from datetime import datetime
from typing import Any

from sqlalchemy import (
    BigInteger,
    DateTime,
    ForeignKey,
    JSON,
    String,
    Text,
    UniqueConstraint,
)
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.db.base import Base


class Creator(Base):
    __tablename__ = "creators"

    id: Mapped[uuid.UUID] = mapped_column(primary_key=True, default=uuid.uuid4)
    slug: Mapped[str] = mapped_column(String(160), unique=True, index=True)
    display_name: Mapped[str] = mapped_column(String(240))
    aliases: Mapped[list[str]] = mapped_column(JSON, default=list)
    categories: Mapped[list[str]] = mapped_column(JSON, default=list)
    features: Mapped[list[str]] = mapped_column(JSON, default=list)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=datetime.now
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=datetime.now, onupdate=datetime.now
    )

    mods: Mapped[list["Mod"]] = relationship(back_populates="creator")


class Mod(Base):
    __tablename__ = "mods"
    __table_args__ = (UniqueConstraint("creator_id", "slug", name="uq_mod_creator_slug"),)

    id: Mapped[uuid.UUID] = mapped_column(primary_key=True, default=uuid.uuid4)
    creator_id: Mapped[uuid.UUID] = mapped_column(
        ForeignKey("creators.id", ondelete="CASCADE"), index=True
    )
    slug: Mapped[str] = mapped_column(String(180))
    name: Mapped[str] = mapped_column(String(280))
    description: Mapped[str | None] = mapped_column(Text)
    aliases: Mapped[list[str]] = mapped_column(JSON, default=list)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=datetime.now
    )

    creator: Mapped[Creator] = relationship(back_populates="mods")
    releases: Mapped[list["ModRelease"]] = relationship(back_populates="mod")


class ModRelease(Base):
    __tablename__ = "mod_releases"
    __table_args__ = (
        UniqueConstraint(
            "source_id",
            "source_record_id",
            name="uq_mod_release_source_record",
        ),
    )

    id: Mapped[uuid.UUID] = mapped_column(primary_key=True, default=uuid.uuid4)
    mod_id: Mapped[uuid.UUID] = mapped_column(
        ForeignKey("mods.id", ondelete="CASCADE"), index=True
    )
    version: Mapped[str | None] = mapped_column(String(160))
    released_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True))
    source_id: Mapped[uuid.UUID | None] = mapped_column(
        ForeignKey("sources.id", ondelete="SET NULL"), index=True
    )
    source_record_id: Mapped[str | None] = mapped_column(String(240))
    source_url: Mapped[str | None] = mapped_column(String(2048))
    retrieved_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True))
    changelog: Mapped[str | None] = mapped_column(Text)

    mod: Mapped[Mod] = relationship(back_populates="releases")
    artifacts: Mapped[list["Artifact"]] = relationship(back_populates="release")


class Artifact(Base):
    __tablename__ = "artifacts"
    __table_args__ = (
        UniqueConstraint(
            "source_id",
            "source_record_id",
            name="uq_artifact_source_record",
        ),
    )

    id: Mapped[uuid.UUID] = mapped_column(primary_key=True, default=uuid.uuid4)
    release_id: Mapped[uuid.UUID] = mapped_column(
        ForeignKey("mod_releases.id", ondelete="CASCADE"), index=True
    )
    artifact_kind: Mapped[str] = mapped_column(String(32))
    filename: Mapped[str | None] = mapped_column(String(512))
    size_bytes: Mapped[int | None] = mapped_column(BigInteger)
    source_id: Mapped[uuid.UUID | None] = mapped_column(
        ForeignKey("sources.id", ondelete="SET NULL"), index=True
    )
    source_record_id: Mapped[str | None] = mapped_column(String(240))
    source_url: Mapped[str | None] = mapped_column(String(2048))
    retrieved_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True))
    metadata_json: Mapped[dict[str, Any]] = mapped_column(JSON, default=dict)

    release: Mapped[ModRelease] = relationship(back_populates="artifacts")
    fingerprints: Mapped[list["Fingerprint"]] = relationship(
        back_populates="artifact"
    )
