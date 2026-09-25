"""Create initial registry domain.

Revision ID: 0001_registry_domain
Revises:
"""

from alembic import op
import sqlalchemy as sa


revision = "0001_registry_domain"
down_revision = None
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "sources",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("kind", sa.String(length=48), nullable=False),
        sa.Column("external_id", sa.String(length=240), nullable=False),
        sa.Column("name", sa.String(length=280), nullable=False),
        sa.Column("base_url", sa.String(length=2048), nullable=True),
        sa.Column("metadata_json", sa.JSON(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint("kind", "external_id", name="uq_source_kind_external"),
    )
    op.create_index("ix_sources_kind", "sources", ["kind"])

    op.create_table(
        "creators",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("slug", sa.String(length=160), nullable=False),
        sa.Column("display_name", sa.String(length=240), nullable=False),
        sa.Column("aliases", sa.JSON(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint("slug"),
    )
    op.create_index("ix_creators_slug", "creators", ["slug"])

    op.create_table(
        "mods",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("creator_id", sa.Uuid(), nullable=False),
        sa.Column("slug", sa.String(length=180), nullable=False),
        sa.Column("name", sa.String(length=280), nullable=False),
        sa.Column("description", sa.Text(), nullable=True),
        sa.Column("aliases", sa.JSON(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["creator_id"], ["creators.id"], ondelete="CASCADE"),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint("creator_id", "slug", name="uq_mod_creator_slug"),
    )
    op.create_index("ix_mods_creator_id", "mods", ["creator_id"])

    op.create_table(
        "mod_releases",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("mod_id", sa.Uuid(), nullable=False),
        sa.Column("version", sa.String(length=160), nullable=True),
        sa.Column("released_at", sa.DateTime(timezone=True), nullable=True),
        sa.Column("source_id", sa.Uuid(), nullable=True),
        sa.Column("source_url", sa.String(length=2048), nullable=True),
        sa.Column("retrieved_at", sa.DateTime(timezone=True), nullable=True),
        sa.Column("changelog", sa.Text(), nullable=True),
        sa.ForeignKeyConstraint(["mod_id"], ["mods.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["source_id"], ["sources.id"], ondelete="SET NULL"),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index("ix_mod_releases_mod_id", "mod_releases", ["mod_id"])
    op.create_index("ix_mod_releases_source_id", "mod_releases", ["source_id"])

    op.create_table(
        "artifacts",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("release_id", sa.Uuid(), nullable=False),
        sa.Column("artifact_kind", sa.String(length=32), nullable=False),
        sa.Column("filename", sa.String(length=512), nullable=True),
        sa.Column("size_bytes", sa.BigInteger(), nullable=True),
        sa.Column("source_id", sa.Uuid(), nullable=True),
        sa.Column("source_url", sa.String(length=2048), nullable=True),
        sa.Column("retrieved_at", sa.DateTime(timezone=True), nullable=True),
        sa.Column("metadata_json", sa.JSON(), nullable=False),
        sa.CheckConstraint(
            "artifact_kind IN ('package', 'ts4script', 'archive', 'unknown')",
            name="ck_artifacts_kind",
        ),
        sa.ForeignKeyConstraint(["release_id"], ["mod_releases.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["source_id"], ["sources.id"], ondelete="SET NULL"),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index("ix_artifacts_release_id", "artifacts", ["release_id"])
    op.create_index("ix_artifacts_source_id", "artifacts", ["source_id"])

    op.create_table(
        "fingerprints",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("artifact_id", sa.Uuid(), nullable=False),
        sa.Column("kind", sa.String(length=48), nullable=False),
        sa.Column("value", sa.String(length=512), nullable=False),
        sa.Column("algorithm_version", sa.String(length=96), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.CheckConstraint(
            "kind IN ('sha256', 'curseforge', 'resource_signature', 'script_signature')",
            name="ck_registry_fingerprint_kind",
        ),
        sa.ForeignKeyConstraint(["artifact_id"], ["artifacts.id"], ondelete="CASCADE"),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint(
            "artifact_id",
            "kind",
            "value",
            "algorithm_version",
            name="uq_registry_fingerprint_artifact_kind_value",
        ),
    )
    op.create_index("ix_fingerprints_artifact_id", "fingerprints", ["artifact_id"])
    op.create_index(
        "ix_registry_fingerprint_lookup",
        "fingerprints",
        ["kind", "value", "algorithm_version"],
    )

    op.create_table(
        "game_patches",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("normalized_version", sa.String(length=96), nullable=False),
        sa.Column("platform", sa.String(length=32), nullable=False),
        sa.Column("released_at", sa.DateTime(timezone=True), nullable=True),
        sa.Column("source_id", sa.Uuid(), nullable=False),
        sa.Column("source_url", sa.String(length=2048), nullable=True),
        sa.Column("retrieved_at", sa.DateTime(timezone=True), nullable=False),
        sa.CheckConstraint("platform IN ('windows')", name="ck_game_patches_platform"),
        sa.ForeignKeyConstraint(["source_id"], ["sources.id"], ondelete="RESTRICT"),
        sa.PrimaryKeyConstraint("id"),
        sa.UniqueConstraint("normalized_version"),
    )
    op.create_index(
        "ix_game_patches_normalized_version",
        "game_patches",
        ["normalized_version"],
    )
    op.create_index("ix_game_patches_source_id", "game_patches", ["source_id"])

    op.create_table(
        "compatibility_reports",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("release_id", sa.Uuid(), nullable=False),
        sa.Column("patch_id", sa.Uuid(), nullable=False),
        sa.Column("status", sa.String(length=48), nullable=False),
        sa.Column("source_id", sa.Uuid(), nullable=False),
        sa.Column("source_url", sa.String(length=2048), nullable=True),
        sa.Column("source_record_id", sa.String(length=240), nullable=True),
        sa.Column("retrieved_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("notes", sa.Text(), nullable=True),
        sa.CheckConstraint(
            "status IN ('compatible', 'update_available', 'unknown', 'potential_conflict', 'broken', 'abandoned')",
            name="ck_compatibility_status",
        ),
        sa.ForeignKeyConstraint(["release_id"], ["mod_releases.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["patch_id"], ["game_patches.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["source_id"], ["sources.id"], ondelete="RESTRICT"),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index(
        "ix_compatibility_reports_release_id",
        "compatibility_reports",
        ["release_id"],
    )
    op.create_index(
        "ix_compatibility_reports_patch_id",
        "compatibility_reports",
        ["patch_id"],
    )
    op.create_index(
        "ix_compatibility_reports_source_id",
        "compatibility_reports",
        ["source_id"],
    )
    op.create_index(
        "ix_compatibility_release_patch_retrieved",
        "compatibility_reports",
        ["release_id", "patch_id", "retrieved_at"],
    )


def downgrade() -> None:
    op.drop_table("compatibility_reports")
    op.drop_table("game_patches")
    op.drop_table("fingerprints")
    op.drop_table("artifacts")
    op.drop_table("mod_releases")
    op.drop_table("mods")
    op.drop_table("creators")
    op.drop_table("sources")
