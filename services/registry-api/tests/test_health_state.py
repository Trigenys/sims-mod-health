from __future__ import annotations

from datetime import datetime, timedelta, timezone

from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from app.domain import (
    CompatibilityReport,
    Creator,
    GamePatch,
    Mod,
    ModRelease,
    Source,
)
from app.main import app


client = TestClient(app)
NOW = datetime(2026, 9, 26, 12, 0, tzinfo=timezone.utc)


def seed_source(
    session: Session,
    external_id: str,
    *,
    kind: str = "test",
) -> Source:
    source = Source(
        kind=kind,
        external_id=external_id,
        name=external_id,
        base_url=f"https://example.invalid/{external_id}",
        metadata_json={},
        created_at=NOW,
    )
    session.add(source)
    session.flush()
    return source


def seed_mod_with_releases(
    session: Session,
    source: Source,
) -> tuple[ModRelease, ModRelease]:
    creator = Creator(
        slug="creator",
        display_name="Creator",
        aliases=["CreatorAlias"],
        created_at=NOW,
        updated_at=NOW,
    )
    session.add(creator)
    session.flush()

    mod = Mod(
        creator_id=creator.id,
        slug="example-mod",
        name="Example Mod",
        aliases=[],
        created_at=NOW,
    )
    session.add(mod)
    session.flush()

    installed = ModRelease(
        mod_id=mod.id,
        version="1.0.0",
        released_at=NOW - timedelta(days=30),
        source_id=source.id,
        source_record_id="release-1",
        source_url="https://example.invalid/release-1",
        retrieved_at=NOW,
    )
    newer = ModRelease(
        mod_id=mod.id,
        version="2.0.0",
        released_at=NOW - timedelta(days=1),
        source_id=source.id,
        source_record_id="release-2",
        source_url="https://example.invalid/release-2",
        retrieved_at=NOW,
    )
    session.add_all([installed, newer])
    session.flush()
    return installed, newer


def seed_patch(
    session: Session,
    source: Source,
    version: str,
) -> GamePatch:
    patch = GamePatch(
        normalized_version=version,
        platform="windows",
        released_at=NOW,
        source_id=source.id,
        source_url=f"https://example.invalid/patch/{version}",
        retrieved_at=NOW,
    )
    session.add(patch)
    session.flush()
    return patch


def add_exact_report(
    session: Session,
    *,
    release: ModRelease,
    patch: GamePatch,
    source: Source,
    status: str,
    retrieved_at: datetime = NOW,
) -> CompatibilityReport:
    report = CompatibilityReport(
        release_id=release.id,
        patch_id=patch.id,
        patch_min_version=None,
        patch_max_version=None,
        status=status,
        source_id=source.id,
        source_url=f"https://example.invalid/evidence/{source.external_id}",
        source_record_id=f"{source.external_id}-{status}",
        retrieved_at=retrieved_at,
        notes=None,
    )
    session.add(report)
    session.flush()
    return report


def add_range_report(
    session: Session,
    *,
    release: ModRelease,
    source: Source,
    status: str,
    minimum: str | None,
    maximum: str | None,
) -> CompatibilityReport:
    report = CompatibilityReport(
        release_id=release.id,
        patch_id=None,
        patch_min_version=minimum,
        patch_max_version=maximum,
        status=status,
        source_id=source.id,
        source_url=f"https://example.invalid/evidence/{source.external_id}",
        source_record_id=f"{source.external_id}-{status}-range",
        retrieved_at=NOW,
        notes=None,
    )
    session.add(report)
    session.flush()
    return report


def evaluate(release: ModRelease, patch_version: str) -> dict:
    response = client.post(
        "/v1/health/evaluate",
        json={
            "patch_version": patch_version,
            "platform": "windows",
            "installed_release_ids": [str(release.id)],
        },
    )
    assert response.status_code == 200
    return response.json()["items"][0]


def test_exact_patch_evidence_is_compatible_and_beats_range(
    db_session: Session,
) -> None:
    source = seed_source(db_session, "catalog")
    range_source = seed_source(db_session, "range")
    installed, newer = seed_mod_with_releases(db_session, source)
    newer.released_at = installed.released_at
    newer.version = installed.version
    patch = seed_patch(db_session, source, "1.128.90.1030")

    add_range_report(
        db_session,
        release=installed,
        source=range_source,
        status="broken",
        minimum="1.120",
        maximum="1.130",
    )
    add_exact_report(
        db_session,
        release=installed,
        patch=patch,
        source=source,
        status="compatible",
    )
    db_session.commit()

    item = evaluate(installed, "1.128.90.1030")

    assert item["state"] == "compatible"
    assert item["compatibility_state"] == "compatible"
    assert item["disputed"] is False
    assert item["reason"] == "exact_patch_evidence"
    assert len(item["evidence"]) == 1
    assert item["evidence"][0]["scope"] == "exact_patch"


def test_new_patch_does_not_inherit_old_compatibility(
    db_session: Session,
) -> None:
    source = seed_source(db_session, "catalog")
    installed, newer = seed_mod_with_releases(db_session, source)
    newer.released_at = installed.released_at
    newer.version = installed.version
    old_patch = seed_patch(db_session, source, "1.127.0.0")
    add_exact_report(
        db_session,
        release=installed,
        patch=old_patch,
        source=source,
        status="compatible",
    )
    db_session.commit()

    item = evaluate(installed, "1.128.0.0")

    assert item["state"] == "unknown"
    assert item["compatibility_state"] == "unknown"
    assert item["disputed"] is False
    assert item["reason"] == "no_current_patch_evidence"
    assert item["evidence"] == []


def test_patch_range_can_cover_new_patch(
    db_session: Session,
) -> None:
    source = seed_source(db_session, "catalog")
    installed, newer = seed_mod_with_releases(db_session, source)
    newer.released_at = installed.released_at
    newer.version = installed.version
    add_range_report(
        db_session,
        release=installed,
        source=source,
        status="compatible",
        minimum="1.126.0.0",
        maximum="1.130.0.0",
    )
    db_session.commit()

    item = evaluate(installed, "1.128.90.1030")

    assert item["state"] == "compatible"
    assert item["reason"] == "patch_range_evidence"
    assert item["evidence"][0]["scope"] == "patch_range"
    assert item["evidence"][0]["patch_min_version"] == "1.126.0.0"
    assert item["evidence"][0]["patch_max_version"] == "1.130.0.0"


def test_conflicting_current_patch_sources_are_disputed_unknown(
    db_session: Session,
) -> None:
    catalog = seed_source(db_session, "catalog")
    source_a = seed_source(db_session, "source-a")
    source_b = seed_source(db_session, "source-b")
    installed, newer = seed_mod_with_releases(db_session, catalog)
    newer.released_at = installed.released_at
    newer.version = installed.version
    patch = seed_patch(db_session, catalog, "1.128.90.1030")
    add_exact_report(
        db_session,
        release=installed,
        patch=patch,
        source=source_a,
        status="compatible",
    )
    add_exact_report(
        db_session,
        release=installed,
        patch=patch,
        source=source_b,
        status="broken",
    )
    db_session.commit()

    item = evaluate(installed, "1.128.90.1030")

    assert item["state"] == "unknown"
    assert item["compatibility_state"] == "unknown"
    assert item["disputed"] is True
    assert item["reason"] == "conflicting_current_patch_evidence"
    assert {evidence["status"] for evidence in item["evidence"]} == {
        "compatible",
        "broken",
    }


def test_update_available_is_independent_from_target_compatibility(
    db_session: Session,
) -> None:
    source = seed_source(db_session, "catalog")
    installed, newer = seed_mod_with_releases(db_session, source)
    patch = seed_patch(db_session, source, "1.128.90.1030")
    add_exact_report(
        db_session,
        release=installed,
        patch=patch,
        source=source,
        status="compatible",
    )
    db_session.commit()

    item = evaluate(installed, "1.128.90.1030")

    assert item["state"] == "update_available"
    assert item["compatibility_state"] == "compatible"
    assert item["update"] == {
        "available": True,
        "target_release_id": str(newer.id),
        "target_version": "2.0.0",
        "target_compatibility_state": "unknown",
        "target_disputed": False,
    }


def test_broken_state_is_not_hidden_by_available_update(
    db_session: Session,
) -> None:
    source = seed_source(db_session, "catalog")
    installed, newer = seed_mod_with_releases(db_session, source)
    patch = seed_patch(db_session, source, "1.128.90.1030")
    add_exact_report(
        db_session,
        release=installed,
        patch=patch,
        source=source,
        status="broken",
    )
    db_session.commit()

    item = evaluate(installed, "1.128.90.1030")

    assert item["state"] == "broken"
    assert item["compatibility_state"] == "broken"
    assert item["update"]["available"] is True
    assert item["update"]["target_release_id"] == str(newer.id)


def test_latest_report_from_same_source_supersedes_older_report(
    db_session: Session,
) -> None:
    source = seed_source(db_session, "catalog")
    installed, newer = seed_mod_with_releases(db_session, source)
    newer.released_at = installed.released_at
    newer.version = installed.version
    patch = seed_patch(db_session, source, "1.128.90.1030")
    add_exact_report(
        db_session,
        release=installed,
        patch=patch,
        source=source,
        status="broken",
        retrieved_at=NOW - timedelta(hours=1),
    )
    add_exact_report(
        db_session,
        release=installed,
        patch=patch,
        source=source,
        status="compatible",
        retrieved_at=NOW,
    )
    db_session.commit()

    item = evaluate(installed, "1.128.90.1030")

    assert item["state"] == "compatible"
    assert item["disputed"] is False
    assert len(item["evidence"]) == 1
    assert item["evidence"][0]["status"] == "compatible"



def test_absent_current_source_evidence_never_manufactures_broken(
    db_session: Session,
) -> None:
    source = seed_source(db_session, "temporarily-unavailable-source")
    installed, newer = seed_mod_with_releases(db_session, source)
    newer.released_at = installed.released_at
    newer.version = installed.version
    db_session.commit()

    item = evaluate(installed, "1.128.90.1030")

    assert item["state"] == "unknown"
    assert item["compatibility_state"] == "unknown"
    assert item["reason"] == "no_current_patch_evidence"
    assert item["evidence"] == []
