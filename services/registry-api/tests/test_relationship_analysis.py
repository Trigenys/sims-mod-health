from __future__ import annotations

from datetime import datetime, timezone

from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from app.domain import (
    ConflictRule,
    Creator,
    DependencyRule,
    Mod,
    ModRelease,
    Source,
)
from app.main import app


client = TestClient(app)
NOW = datetime(2026, 9, 26, 12, 0, tzinfo=timezone.utc)


def seed_release(
    session: Session,
    name: str,
    version: str,
    *,
    source_kind: str = "test",
    source_external_id: str | None = None,
) -> tuple[Source, Mod, ModRelease]:
    slug = name.lower().replace(" ", "-")
    source = Source(
        kind=source_kind,
        external_id=source_external_id or f"{slug}-source",
        name=name,
        base_url=f"https://example.invalid/{slug}",
        metadata_json={},
        created_at=NOW,
    )
    creator = Creator(
        slug=f"{slug}-creator",
        display_name=f"{name} Creator",
        aliases=[],
        created_at=NOW,
        updated_at=NOW,
    )
    session.add_all([source, creator])
    session.flush()

    mod = Mod(
        creator_id=creator.id,
        slug=slug,
        name=name,
        aliases=[],
        created_at=NOW,
    )
    session.add(mod)
    session.flush()

    release = ModRelease(
        mod_id=mod.id,
        version=version,
        released_at=NOW,
        source_id=source.id,
        source_record_id=f"{slug}-{version}",
        source_url=f"https://example.invalid/{slug}/{version}",
        retrieved_at=NOW,
    )
    session.add(release)
    session.flush()
    return source, mod, release


def add_dependency(
    session: Session,
    *,
    owner: ModRelease,
    provenance: Source,
    target_mod: Mod | None = None,
    target_source_kind: str | None = None,
    target_source_external_id: str | None = None,
    minimum: str | None = None,
    maximum: str | None = None,
) -> DependencyRule:
    rule = DependencyRule(
        release_id=owner.id,
        target_mod_id=target_mod.id if target_mod is not None else None,
        target_source_kind=target_source_kind,
        target_source_external_id=target_source_external_id,
        min_version=minimum,
        max_version=maximum,
        source_id=provenance.id,
        source_url=provenance.base_url,
        source_record_id=f"dependency-{owner.id}",
        retrieved_at=NOW,
        notes="test dependency",
    )
    session.add(rule)
    session.flush()
    return rule


def add_conflict(
    session: Session,
    *,
    owner: ModRelease,
    provenance: Source,
    target_mod: Mod,
    minimum: str | None = None,
    maximum: str | None = None,
) -> ConflictRule:
    rule = ConflictRule(
        release_id=owner.id,
        target_mod_id=target_mod.id,
        target_source_kind=None,
        target_source_external_id=None,
        min_version=minimum,
        max_version=maximum,
        source_id=provenance.id,
        source_url=provenance.base_url,
        source_record_id=f"conflict-{owner.id}",
        retrieved_at=NOW,
        notes="known incompatibility",
    )
    session.add(rule)
    session.flush()
    return rule


def analyze(*releases: ModRelease) -> dict:
    response = client.post(
        "/v1/health/relationships/evaluate",
        json={
            "installed_release_ids": [str(release.id) for release in releases]
        },
    )
    assert response.status_code == 200
    return response.json()


def test_missing_and_outdated_dependencies_are_actionable(
    db_session: Session,
) -> None:
    provenance, _, owner = seed_release(db_session, "Owner", "1.0")
    _, dependency_mod, dependency = seed_release(db_session, "Dependency", "1.0")
    _, missing_mod, _missing_release = seed_release(db_session, "Missing", "1.0")

    add_dependency(
        db_session,
        owner=owner,
        provenance=provenance,
        target_mod=dependency_mod,
        minimum="2.0",
    )
    add_dependency(
        db_session,
        owner=owner,
        provenance=provenance,
        target_mod=missing_mod,
    )
    db_session.commit()

    payload = analyze(owner, dependency)
    findings = payload["dependency_findings"]

    assert {(item["status"], item["action"]) for item in findings} == {
        ("outdated", "update_dependency"),
        ("missing", "install_dependency"),
    }
    outdated = next(item for item in findings if item["status"] == "outdated")
    assert outdated["installed_target_release_id"] == str(dependency.id)
    assert outdated["installed_target_version"] == "1.0"


def test_shared_dependency_exposes_reverse_usage_count(
    db_session: Session,
) -> None:
    provenance, _, first = seed_release(db_session, "First", "1.0")
    _, _, second = seed_release(db_session, "Second", "1.0")
    _, shared_mod, shared = seed_release(db_session, "Shared", "3.0")

    add_dependency(
        db_session,
        owner=first,
        provenance=provenance,
        target_mod=shared_mod,
    )
    add_dependency(
        db_session,
        owner=second,
        provenance=provenance,
        target_mod=shared_mod,
    )
    db_session.commit()

    payload = analyze(first, second, shared)

    assert payload["dependency_findings"] == []
    assert payload["reverse_usage"] == [
        {
            "dependency_release_id": str(shared.id),
            "used_by_count": 2,
            "used_by_release_ids": sorted(
                [str(first.id), str(second.id)]
            ),
        }
    ]


def test_dependency_cycles_are_reported_without_recursion_failure(
    db_session: Session,
) -> None:
    provenance, first_mod, first = seed_release(db_session, "Cycle A", "1.0")
    _, second_mod, second = seed_release(db_session, "Cycle B", "1.0")

    add_dependency(
        db_session,
        owner=first,
        provenance=provenance,
        target_mod=second_mod,
    )
    add_dependency(
        db_session,
        owner=second,
        provenance=provenance,
        target_mod=first_mod,
    )
    db_session.commit()

    payload = analyze(first, second)

    assert payload["dependency_findings"] == []
    assert payload["cycles"] == [
        {"release_ids": sorted([str(first.id), str(second.id)])}
    ]


def test_version_constrained_incompatibility_retains_provenance(
    db_session: Session,
) -> None:
    provenance, _, owner = seed_release(db_session, "Owner Conflict", "1.0")
    _, target_mod, target = seed_release(db_session, "Target Conflict", "2.5")

    rule = add_conflict(
        db_session,
        owner=owner,
        provenance=provenance,
        target_mod=target_mod,
        minimum="2.0",
        maximum="3.0",
    )
    db_session.commit()

    payload = analyze(owner, target)

    assert len(payload["known_incompatibilities"]) == 1
    finding = payload["known_incompatibilities"][0]
    assert finding["rule_id"] == str(rule.id)
    assert finding["left_release_id"] == str(owner.id)
    assert finding["right_release_id"] == str(target.id)
    assert finding["right_version"] == "2.5"
    assert finding["target"]["min_version"] == "2.0"
    assert finding["target"]["max_version"] == "3.0"
    assert finding["provenance"]["source_id"] == str(provenance.id)
    assert finding["provenance"]["source_url"] == provenance.base_url


def test_incompatibility_outside_version_range_is_not_claimed(
    db_session: Session,
) -> None:
    provenance, _, owner = seed_release(db_session, "Owner Safe", "1.0")
    _, target_mod, target = seed_release(db_session, "Target Safe", "4.0")

    add_conflict(
        db_session,
        owner=owner,
        provenance=provenance,
        target_mod=target_mod,
        minimum="2.0",
        maximum="3.0",
    )
    db_session.commit()

    payload = analyze(owner, target)

    assert payload["known_incompatibilities"] == []


def test_source_identity_dependency_matches_installed_release(
    db_session: Session,
) -> None:
    provenance, _, owner = seed_release(db_session, "CF Owner", "1.0")
    _, _, target = seed_release(
        db_session,
        "CF Target",
        "1.0",
        source_kind="curseforge",
        source_external_id="77",
    )

    add_dependency(
        db_session,
        owner=owner,
        provenance=provenance,
        target_source_kind="curseforge",
        target_source_external_id="77",
    )
    db_session.commit()

    payload = analyze(owner, target)

    assert payload["dependency_findings"] == []
    assert payload["reverse_usage"][0]["dependency_release_id"] == str(target.id)
    assert payload["reverse_usage"][0]["used_by_count"] == 1
