"""Add patch-range compatibility evidence.

Revision ID: 0003_compatibility_patch_ranges
Revises: 0002_curseforge_source_identity
"""

from alembic import op
import sqlalchemy as sa


revision = "0003_compatibility_patch_ranges"
down_revision = "0002_curseforge_source_identity"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.add_column(
        "compatibility_reports",
        sa.Column("patch_min_version", sa.String(length=96), nullable=True),
    )
    op.add_column(
        "compatibility_reports",
        sa.Column("patch_max_version", sa.String(length=96), nullable=True),
    )
    op.alter_column(
        "compatibility_reports",
        "patch_id",
        existing_type=sa.Uuid(),
        nullable=True,
    )
    op.create_check_constraint(
        "ck_compatibility_patch_scope",
        "compatibility_reports",
        "("
        "(patch_id IS NOT NULL AND patch_min_version IS NULL AND patch_max_version IS NULL)"
        " OR "
        "(patch_id IS NULL AND (patch_min_version IS NOT NULL OR patch_max_version IS NOT NULL))"
        ")",
    )


def downgrade() -> None:
    op.execute("DELETE FROM compatibility_reports WHERE patch_id IS NULL")
    op.drop_constraint(
        "ck_compatibility_patch_scope",
        "compatibility_reports",
        type_="check",
    )
    op.alter_column(
        "compatibility_reports",
        "patch_id",
        existing_type=sa.Uuid(),
        nullable=False,
    )
    op.drop_column("compatibility_reports", "patch_max_version")
    op.drop_column("compatibility_reports", "patch_min_version")
