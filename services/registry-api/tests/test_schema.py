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
        "dependency_rules",
        "conflict_rules",
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
    dependency_columns = {
        column["name"]: column
        for column in inspector.get_columns("dependency_rules")
    }
    conflict_columns = {
        column["name"]: column
        for column in inspector.get_columns("conflict_rules")
    }

    for columns in [
        patch_columns,
        report_columns,
        dependency_columns,
        conflict_columns,
    ]:
        assert columns["source_id"]["nullable"] is False
        assert columns["retrieved_at"]["nullable"] is False


def test_compatibility_report_supports_exact_patch_or_range_scope() -> None:
    columns = {
        column["name"]: column
        for column in inspect(engine).get_columns("compatibility_reports")
    }

    assert columns["patch_id"]["nullable"] is True
    assert columns["patch_min_version"]["nullable"] is True
    assert columns["patch_max_version"]["nullable"] is True


def test_relationship_rules_retain_version_constraints_and_source_targets() -> None:
    inspector = inspect(engine)

    for table in ["dependency_rules", "conflict_rules"]:
        columns = {
            column["name"]
            for column in inspector.get_columns(table)
        }
        assert {
            "release_id",
            "target_mod_id",
            "target_source_kind",
            "target_source_external_id",
            "min_version",
            "max_version",
            "source_id",
            "source_record_id",
            "retrieved_at",
        }.issubset(columns)



def test_mods_include_deterministic_discovery_taxonomy() -> None:
    columns = {
        column["name"]: column
        for column in inspect(engine).get_columns("mods")
    }

    assert columns["categories"]["nullable"] is False
    assert columns["features"]["nullable"] is False
