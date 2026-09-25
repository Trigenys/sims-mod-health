from __future__ import annotations

from sqlalchemy import inspect

from app.db.session import engine


def test_migrated_schema_contains_registry_core_entities() -> None:
    tables = set(inspect(engine).get_table_names())

    assert {
        "creators",
        "mods",
        "mod_releases",
        "artifacts",
        "fingerprints",
        "sources",
        "game_patches",
        "compatibility_reports",
    }.issubset(tables)


def test_health_affecting_tables_require_provenance() -> None:
    inspector = inspect(engine)

    patch_columns = {
        column["name"]: column
        for column in inspector.get_columns("game_patches")
    }
    report_columns = {
        column["name"]: column
        for column in inspector.get_columns("compatibility_reports")
    }

    for columns in [patch_columns, report_columns]:
        assert columns["source_id"]["nullable"] is False
        assert columns["retrieved_at"]["nullable"] is False
