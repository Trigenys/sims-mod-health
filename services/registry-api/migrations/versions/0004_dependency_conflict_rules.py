"""Add dependency and incompatibility rules.

Revision ID: 0004_dependency_conflict_rules
Revises: 0003_compatibility_patch_ranges
"""

from alembic import op
import sqlalchemy as sa


revision = "0004_dependency_conflict_rules"
down_revision = "0003_compatibility_patch_ranges"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "dependency_rules",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("release_id", sa.Uuid(), nullable=False),
        sa.Column("target_mod_id", sa.Uuid(), nullable=True),
        sa.Column("target_source_kind", sa.String(length=48), nullable=True),
        sa.Column("target_source_external_id", sa.String(length=240), nullable=True),
        sa.Column("min_version", sa.String(length=160), nullable=True),
        sa.Column("max_version", sa.String(length=160), nullable=True),
        sa.Column("source_id", sa.Uuid(), nullable=False),
        sa.Column("source_url", sa.String(length=2048), nullable=True),
        sa.Column("source_record_id", sa.String(length=240), nullable=True),
        sa.Column("retrieved_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("notes", sa.Text(), nullable=True),
        sa.CheckConstraint(
            "target_mod_id IS NOT NULL OR "
            "(target_source_kind IS NOT NULL AND target_source_external_id IS NOT NULL)",
            name="ck_dependency_rule_target",
        ),
        sa.ForeignKeyConstraint(
            ["release_id"], ["mod_releases.id"], ondelete="CASCADE"
        ),
        sa.ForeignKeyConstraint(
            ["target_mod_id"], ["mods.id"], ondelete="CASCADE"
        ),
        sa.ForeignKeyConstraint(
            ["source_id"], ["sources.id"], ondelete="RESTRICT"
        ),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index(
        "ix_dependency_rules_release_id",
        "dependency_rules",
        ["release_id"],
    )
    op.create_index(
        "ix_dependency_rules_target_mod_id",
        "dependency_rules",
        ["target_mod_id"],
    )
    op.create_index(
        "ix_dependency_rules_target_source",
        "dependency_rules",
        ["target_source_kind", "target_source_external_id"],
    )
    op.create_index(
        "ix_dependency_rules_source_id",
        "dependency_rules",
        ["source_id"],
    )

    op.create_table(
        "conflict_rules",
        sa.Column("id", sa.Uuid(), nullable=False),
        sa.Column("release_id", sa.Uuid(), nullable=False),
        sa.Column("target_mod_id", sa.Uuid(), nullable=True),
        sa.Column("target_source_kind", sa.String(length=48), nullable=True),
        sa.Column("target_source_external_id", sa.String(length=240), nullable=True),
        sa.Column("min_version", sa.String(length=160), nullable=True),
        sa.Column("max_version", sa.String(length=160), nullable=True),
        sa.Column("source_id", sa.Uuid(), nullable=False),
        sa.Column("source_url", sa.String(length=2048), nullable=True),
        sa.Column("source_record_id", sa.String(length=240), nullable=True),
        sa.Column("retrieved_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("notes", sa.Text(), nullable=True),
        sa.CheckConstraint(
            "target_mod_id IS NOT NULL OR "
            "(target_source_kind IS NOT NULL AND target_source_external_id IS NOT NULL)",
            name="ck_conflict_rule_target",
        ),
        sa.ForeignKeyConstraint(
            ["release_id"], ["mod_releases.id"], ondelete="CASCADE"
        ),
        sa.ForeignKeyConstraint(
            ["target_mod_id"], ["mods.id"], ondelete="CASCADE"
        ),
        sa.ForeignKeyConstraint(
            ["source_id"], ["sources.id"], ondelete="RESTRICT"
        ),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index(
        "ix_conflict_rules_release_id",
        "conflict_rules",
        ["release_id"],
    )
    op.create_index(
        "ix_conflict_rules_target_mod_id",
        "conflict_rules",
        ["target_mod_id"],
    )
    op.create_index(
        "ix_conflict_rules_target_source",
        "conflict_rules",
        ["target_source_kind", "target_source_external_id"],
    )
    op.create_index(
        "ix_conflict_rules_source_id",
        "conflict_rules",
        ["source_id"],
    )


def downgrade() -> None:
    op.drop_table("conflict_rules")
    op.drop_table("dependency_rules")
