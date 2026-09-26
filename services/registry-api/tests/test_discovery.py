from __future__ import annotations

from datetime import datetime, timezone

from sqlalchemy.orm import Session

from app.domain import CompatibilityReport, ConflictRule, Creator, Mod, ModRelease, Source
from app.repositories.discovery import DiscoveryRepository
from app.repositories.health_state import HealthStateRepository
from app.repositories.relationship_analysis import RelationshipAnalysisRepository
from app.schemas.discovery import DiscoveryRecommendRequest
from app.services.discovery import DiscoveryRecommendationService
from app.services.discovery_taxonomy import infer_taxonomy


NOW = datetime(2026, 9, 27, tzinfo=timezone.utc)
PATCH = "1.128.90"


def add_source(session: Session) -> Source:
    source = Source(
        kind="test",
        external_id="source",
        name="Test source",
        base_url="https://example.test",
        metadata_json={},
    )
    session.add(source)
    session.flush()
    return source


def add_mod(
    session: Session,
    *,
    name: str,
    slug: str,
    categories: list[str],
    features: list[str],
) -> tuple[Mod, ModRelease]:
    creator = Creator(
        slug=f"creator-{slug}",
        display_name=f"Creator {name}",
        aliases=[],
    )
    session.add(creator)
    session.flush()

    mod = Mod(
        creator_id=creator.id,
        slug=slug,
        name=name,
        description=None,
        aliases=[],
        categories=categories,
        features=features,
    )
    session.add(mod)
    session.flush()

    release = ModRelease(
        mod_id=mod.id,
        version="1.0.0",
        released_at=NOW,
    )
    session.add(release)
    session.flush()
    return mod, release


def compatible(
    session: Session,
    source: Source,
    release: ModRelease,
    *,
    status: str = "compatible",
) -> None:
    if release.id is None:
        session.flush()

    session.add(
        CompatibilityReport(
            release_id=release.id,
            patch_id=None,
            patch_min_version=PATCH,
            patch_max_version=PATCH,
            status=status,
            source_id=source.id,
            source_url="https://example.test/evidence",
            source_record_id=f"compat-{release.id}",
            retrieved_at=NOW,
            notes=None,
        )
    )


def service(session: Session) -> DiscoveryRecommendationService:
    return DiscoveryRecommendationService(
        DiscoveryRepository(session),
        HealthStateRepository(session),
        RelationshipAnalysisRepository(session),
    )


def request(installed_release_id) -> DiscoveryRecommendRequest:
    return DiscoveryRecommendRequest(
        patch_version=PATCH,
        installed_release_ids=[installed_release_id],
        limit=20,
    )


def test_ranking_is_deterministic_and_explains_the_anchor(db_session: Session) -> None:
    source = add_source(db_session)
    _, installed = add_mod(
        db_session,
        name="MC Command Center",
        slug="mccc",
        categories=["gameplay", "story-progression"],
        features=["population-management", "cheats"],
    )
    _, population = add_mod(
        db_session,
        name="Population Tweaks",
        slug="population-tweaks",
        categories=["gameplay"],
        features=["population-management"],
    )
    _, story = add_mod(
        db_session,
        name="Story Plus",
        slug="story-plus",
        categories=["story-progression"],
        features=["population-management"],
    )
    _, unrelated = add_mod(
        db_session,
        name="CAS Lamp",
        slug="cas-lamp",
        categories=["cas"],
        features=["lighting"],
    )

    for release in [population, story, unrelated]:
        compatible(db_session, source, release)
    db_session.commit()

    first = service(db_session).recommend(request(installed.id))
    second = service(db_session).recommend(request(installed.id))

    assert [item.name for item in first.recommendations] == [
        "Population Tweaks",
        "Story Plus",
    ]
    assert first.model_dump() == second.model_dump()
    assert first.recommendations[0].score == 14
    assert first.recommendations[0].reason.because_mod_name == "MC Command Center"
    assert first.recommendations[0].reason.explanation.startswith(
        "Because you use MC Command Center:"
    )
    assert "sponsored" not in first.recommendations[0].model_dump()


def test_unsafe_candidates_are_filtered_before_ranking(db_session: Session) -> None:
    source = add_source(db_session)
    installed_mod, installed = add_mod(
        db_session,
        name="Installed Gameplay",
        slug="installed",
        categories=["gameplay"],
        features=["autonomy"],
    )
    _, safe = add_mod(
        db_session,
        name="Safe Candidate",
        slug="safe",
        categories=["gameplay"],
        features=["autonomy"],
    )
    _, abandoned = add_mod(
        db_session,
        name="Abandoned Candidate",
        slug="abandoned",
        categories=["gameplay"],
        features=["autonomy"],
    )
    _, unknown = add_mod(
        db_session,
        name="Unknown Candidate",
        slug="unknown",
        categories=["gameplay"],
        features=["autonomy"],
    )
    _, conflicting = add_mod(
        db_session,
        name="Conflict Candidate",
        slug="conflict",
        categories=["gameplay"],
        features=["autonomy"],
    )

    compatible(db_session, source, safe)
    compatible(db_session, source, abandoned, status="abandoned")
    compatible(db_session, source, conflicting)
    db_session.add(
        ConflictRule(
            release_id=conflicting.id,
            target_mod_id=installed_mod.id,
            target_source_kind=None,
            target_source_external_id=None,
            min_version=None,
            max_version=None,
            source_id=source.id,
            source_url="https://example.test/conflict",
            source_record_id="conflict-rule",
            retrieved_at=NOW,
            notes="Known incompatibility",
        )
    )
    db_session.commit()

    result = service(db_session).recommend(request(installed.id))

    assert [item.name for item in result.recommendations] == ["Safe Candidate"]
    assert all(item.name != "Abandoned Candidate" for item in result.recommendations)
    assert all(item.name != "Unknown Candidate" for item in result.recommendations)
    assert all(item.name != "Conflict Candidate" for item in result.recommendations)


def test_already_installed_mod_is_never_recommended(db_session: Session) -> None:
    source = add_source(db_session)
    installed_mod, installed = add_mod(
        db_session,
        name="Already Here",
        slug="already-here",
        categories=["gameplay"],
        features=["autonomy"],
    )
    newer = ModRelease(
        mod_id=installed_mod.id,
        version="2.0.0",
        released_at=datetime(2026, 9, 28, tzinfo=timezone.utc),
    )
    db_session.add(newer)
    compatible(db_session, source, newer)

    _, other = add_mod(
        db_session,
        name="Other Mod",
        slug="other",
        categories=["gameplay"],
        features=["autonomy"],
    )
    compatible(db_session, source, other)
    db_session.commit()

    result = service(db_session).recommend(request(installed.id))

    assert [item.name for item in result.recommendations] == ["Other Mod"]


def test_taxonomy_inference_is_deterministic_and_conservative() -> None:
    first = infer_taxonomy(
        name="Story Progression Relationship Tools",
        summary="Population and relationship gameplay utilities.",
        slug="story-progression-tools",
    )
    second = infer_taxonomy(
        name="Story Progression Relationship Tools",
        summary="Population and relationship gameplay utilities.",
        slug="story-progression-tools",
    )

    assert first == second
    categories, features = first
    assert "story-progression" in categories
    assert "relationships" in categories
    assert "population-management" in features
    assert "relationship-management" in features
