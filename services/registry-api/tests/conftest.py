from __future__ import annotations

import pytest
from sqlalchemy import text
from sqlalchemy.orm import Session

from app.db.session import SessionFactory, engine


TRUNCATE_SQL = """
TRUNCATE TABLE
    game_content_manifests,
    conflict_rules,
    dependency_rules,
    compatibility_reports,
    game_patches,
    fingerprints,
    artifacts,
    mod_releases,
    mods,
    creators,
    sources
CASCADE
"""


@pytest.fixture(autouse=True)
def clean_registry_database() -> None:
    with engine.begin() as connection:
        connection.execute(text(TRUNCATE_SQL))

    yield

    with engine.begin() as connection:
        connection.execute(text(TRUNCATE_SQL))


@pytest.fixture
def db_session() -> Session:
    session = SessionFactory()
    try:
        yield session
    finally:
        session.rollback()
        session.close()
