"""Add source record identifiers and source hash kinds.

Revision ID: 0002_curseforge_source_identity
Revises: 0001_registry_domain
"""

from alembic import op
import sqlalchemy as sa


revision = "0002_curseforge_source_identity"
down_revision = "0001_registry_domain"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.add_column(
        "mod_releases",
        sa.Column("source_record_id", sa.String(length=240), nullable=True),
    )
    op.create_unique_constraint(
        "uq_mod_release_source_record",
        "mod_releases",
        ["source_id", "source_record_id"],
    )

    op.add_column(
        "artifacts",
        sa.Column("source_record_id", sa.String(length=240), nullable=True),
    )
    op.create_unique_constraint(
        "uq_artifact_source_record",
        "artifacts",
        ["source_id", "source_record_id"],
    )

    op.drop_constraint(
        "ck_registry_fingerprint_kind",
        "fingerprints",
        type_="check",
    )
    op.create_check_constraint(
        "ck_registry_fingerprint_kind",
        "fingerprints",
        "kind IN ('sha256', 'sha1', 'md5', 'curseforge', 'resource_signature', 'script_signature')",
    )


def downgrade() -> None:
    op.execute("DELETE FROM fingerprints WHERE kind IN ('sha1', 'md5')")

    op.drop_constraint(
        "ck_registry_fingerprint_kind",
        "fingerprints",
        type_="check",
    )
    op.create_check_constraint(
        "ck_registry_fingerprint_kind",
        "fingerprints",
        "kind IN ('sha256', 'curseforge', 'resource_signature', 'script_signature')",
    )

    op.drop_constraint(
        "uq_artifact_source_record",
        "artifacts",
        type_="unique",
    )
    op.drop_column("artifacts", "source_record_id")

    op.drop_constraint(
        "uq_mod_release_source_record",
        "mod_releases",
        type_="unique",
    )
    op.drop_column("mod_releases", "source_record_id")
