"""Add deterministic discovery taxonomy to mods.

Revision ID: 0005_discovery_taxonomy
Revises: 0004_dependency_conflict_rules
"""

from alembic import op
import sqlalchemy as sa


revision = "0005_discovery_taxonomy"
down_revision = "0004_dependency_conflict_rules"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.add_column(
        "mods",
        sa.Column("categories", sa.JSON(), nullable=False, server_default=sa.text("'[]'::json")),
    )
    op.add_column(
        "mods",
        sa.Column("features", sa.JSON(), nullable=False, server_default=sa.text("'[]'::json")),
    )


def downgrade() -> None:
    op.drop_column("mods", "features")
    op.drop_column("mods", "categories")
