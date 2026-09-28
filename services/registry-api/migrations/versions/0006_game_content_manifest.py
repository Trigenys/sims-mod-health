"""Add provenance-bearing Game/DLC manifest documents.

Revision ID: 0006_game_content_manifest
Revises: 0005_discovery_taxonomy
"""

from alembic import op
import sqlalchemy as sa


revision = "0006_game_content_manifest"
down_revision = "0005_discovery_taxonomy"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "game_content_manifests",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("schema_version", sa.Integer(), nullable=False),
        sa.Column("manifest_version", sa.String(length=96), nullable=False),
        sa.Column("source_identity", sa.String(length=240), nullable=False),
        sa.Column("source_url", sa.String(length=2048), nullable=True),
        sa.Column("retrieved_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("expires_at", sa.DateTime(timezone=True), nullable=True),
        sa.Column("checksum_sha256", sa.String(length=64), nullable=True),
        sa.Column("signature", sa.String(length=4096), nullable=True),
        sa.Column("payload_json", sa.JSON(), nullable=False),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint(
            "source_identity",
            "manifest_version",
            name="uq_game_content_manifest_source_version",
        ),
    )
    op.create_index(
        "ix_game_content_manifests_source_identity",
        "game_content_manifests",
        ["source_identity"],
        unique=False,
    )
    op.create_index(
        "ix_game_content_manifests_retrieved_at",
        "game_content_manifests",
        ["retrieved_at"],
        unique=False,
    )


def downgrade() -> None:
    op.drop_index(
        "ix_game_content_manifests_retrieved_at",
        table_name="game_content_manifests",
    )
    op.drop_index(
        "ix_game_content_manifests_source_identity",
        table_name="game_content_manifests",
    )
    op.drop_table("game_content_manifests")
