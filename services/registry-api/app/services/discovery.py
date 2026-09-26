from __future__ import annotations

import re
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone

from app.repositories.discovery import (
    DiscoveryModRecord,
    DiscoveryRepository,
    InstalledDiscoveryRecord,
)
from app.repositories.health_state import HealthStateRepository, ReleaseRecord
from app.repositories.relationship_analysis import (
    InstalledReleaseRecord,
    RelationshipAnalysisRepository,
    RelationshipRuleRecord,
)
from app.schemas.discovery import (
    DiscoveryRecommendation,
    DiscoveryRecommendRequest,
    DiscoveryRecommendResponse,
    RecommendationReason,
)
from app.schemas.health import HealthEvaluateRequest
from app.services.health_state import HealthStateService


HEALTH_BATCH = 100
FEATURE_WEIGHT = 10
CATEGORY_WEIGHT = 4


class UnknownDiscoveryReleaseError(ValueError):
    def __init__(self, release_ids: list[uuid.UUID]) -> None:
        self.release_ids = release_ids
        super().__init__(
            "Unknown installed release ids: "
            + ", ".join(str(item) for item in release_ids)
        )


@dataclass(frozen=True, slots=True)
class CandidateAnchor:
    installed: InstalledDiscoveryRecord
    score: int
    shared_categories: tuple[str, ...]
    shared_features: tuple[str, ...]


class DiscoveryRecommendationService:
    def __init__(
        self,
        discovery_repository: DiscoveryRepository,
        health_repository: HealthStateRepository,
        relationship_repository: RelationshipAnalysisRepository,
    ) -> None:
        self._discovery = discovery_repository
        self._health = health_repository
        self._relationships = relationship_repository

    def recommend(
        self,
        request: DiscoveryRecommendRequest,
    ) -> DiscoveryRecommendResponse:
        requested_ids = set(request.installed_release_ids)
        installed = self._discovery.get_installed(requested_ids)
        missing = [
            release_id
            for release_id in request.installed_release_ids
            if release_id not in installed
        ]
        if missing:
            raise UnknownDiscoveryReleaseError(missing)

        installed_mod_ids = {
            item.mod.mod_id
            for item in installed.values()
        }
        candidates = [
            candidate
            for candidate in self._discovery.list_candidate_mods(installed_mod_ids)
            if _taxonomy(candidate)
        ]
        if not candidates:
            return DiscoveryRecommendResponse(
                patch_version=request.patch_version,
                platform=request.platform,
                recommendations=[],
            )

        candidate_by_mod = {
            candidate.mod_id: candidate
            for candidate in candidates
        }
        releases = self._health.list_releases_for_mods(set(candidate_by_mod))
        latest_by_mod = _latest_release_by_mod(releases)
        latest_release_ids = [
            latest_by_mod[mod_id].id
            for mod_id in sorted(latest_by_mod, key=str)
        ]

        compatibility = self._compatible_candidate_releases(
            request,
            latest_release_ids,
        )
        if not compatibility:
            return DiscoveryRecommendResponse(
                patch_version=request.patch_version,
                platform=request.platform,
                recommendations=[],
            )

        active_candidate_releases = {
            release_id
            for release_id, compatible in compatibility.items()
            if compatible
        }
        conflict_blocked = self._known_conflict_candidates(
            set(request.installed_release_ids),
            active_candidate_releases,
        )

        recommendations: list[DiscoveryRecommendation] = []
        installed_records = list(installed.values())

        for mod_id, candidate in candidate_by_mod.items():
            release = latest_by_mod.get(mod_id)
            if release is None:
                continue
            if release.id not in active_candidate_releases:
                continue
            if release.id in conflict_blocked:
                continue

            anchor = _best_anchor(candidate, installed_records)
            if anchor is None or anchor.score <= 0:
                continue

            categories, features = _taxonomy(candidate)
            recommendations.append(
                DiscoveryRecommendation(
                    mod_id=candidate.mod_id,
                    release_id=release.id,
                    name=candidate.name,
                    creator_name=candidate.creator_name,
                    categories=list(categories),
                    features=list(features),
                    score=anchor.score,
                    reason=RecommendationReason(
                        because_mod_id=anchor.installed.mod.mod_id,
                        because_mod_name=anchor.installed.mod.name,
                        shared_categories=list(anchor.shared_categories),
                        shared_features=list(anchor.shared_features),
                        explanation=_reason_text(candidate, anchor),
                    ),
                )
            )

        recommendations.sort(
            key=lambda item: (
                -item.score,
                item.name.casefold(),
                str(item.mod_id),
                str(item.release_id),
            )
        )

        return DiscoveryRecommendResponse(
            patch_version=request.patch_version,
            platform=request.platform,
            recommendations=recommendations[: request.limit],
        )

    def _compatible_candidate_releases(
        self,
        request: DiscoveryRecommendRequest,
        release_ids: list[uuid.UUID],
    ) -> dict[uuid.UUID, bool]:
        health = HealthStateService(self._health)
        compatible: dict[uuid.UUID, bool] = {}

        for offset in range(0, len(release_ids), HEALTH_BATCH):
            chunk = release_ids[offset : offset + HEALTH_BATCH]
            response = health.evaluate(
                HealthEvaluateRequest(
                    patch_version=request.patch_version,
                    platform=request.platform,
                    installed_release_ids=chunk,
                )
            )
            for item in response.items:
                compatible[item.release_id] = (
                    item.compatibility_state == "compatible"
                    and item.state
                    not in {
                        "broken",
                        "abandoned",
                        "potential_conflict",
                    }
                )

        return compatible

    def _known_conflict_candidates(
        self,
        installed_release_ids: set[uuid.UUID],
        candidate_release_ids: set[uuid.UUID],
    ) -> set[uuid.UUID]:
        if not candidate_release_ids:
            return set()

        all_release_ids = installed_release_ids | candidate_release_ids
        records = self._relationships.get_installed_releases(all_release_ids)
        rules = self._relationships.list_conflict_rules(all_release_ids)
        blocked: set[uuid.UUID] = set()

        installed_records = [
            records[release_id]
            for release_id in installed_release_ids
            if release_id in records
        ]
        candidate_records = [
            records[release_id]
            for release_id in candidate_release_ids
            if release_id in records
        ]

        for rule in rules:
            if rule.release_id in candidate_release_ids:
                for installed in installed_records:
                    if _rule_targets(rule, installed):
                        blocked.add(rule.release_id)
                        break
            elif rule.release_id in installed_release_ids:
                for candidate in candidate_records:
                    if _rule_targets(rule, candidate):
                        blocked.add(candidate.release_id)

        return blocked


def _taxonomy(mod: DiscoveryModRecord) -> tuple[tuple[str, ...], tuple[str, ...]]:
    categories = _normalize_tags(mod.categories)
    features = _normalize_tags(mod.features)
    return categories, features


def _normalize_tags(values: tuple[str, ...]) -> tuple[str, ...]:
    return tuple(
        sorted(
            {
                value.strip().casefold()
                for value in values
                if isinstance(value, str) and value.strip()
            }
        )
    )


def _best_anchor(
    candidate: DiscoveryModRecord,
    installed: list[InstalledDiscoveryRecord],
) -> CandidateAnchor | None:
    candidate_categories, candidate_features = _taxonomy(candidate)
    if not candidate_categories and not candidate_features:
        return None

    candidate_category_set = set(candidate_categories)
    candidate_feature_set = set(candidate_features)
    anchors: list[CandidateAnchor] = []

    for installed_record in installed:
        installed_categories, installed_features = _taxonomy(installed_record.mod)
        shared_categories = tuple(
            sorted(candidate_category_set & set(installed_categories))
        )
        shared_features = tuple(
            sorted(candidate_feature_set & set(installed_features))
        )
        score = (
            len(shared_features) * FEATURE_WEIGHT
            + len(shared_categories) * CATEGORY_WEIGHT
        )
        if score <= 0:
            continue
        anchors.append(
            CandidateAnchor(
                installed=installed_record,
                score=score,
                shared_categories=shared_categories,
                shared_features=shared_features,
            )
        )

    if not anchors:
        return None

    return min(
        anchors,
        key=lambda item: (
            -item.score,
            item.installed.mod.name.casefold(),
            str(item.installed.mod.mod_id),
            str(item.installed.release_id),
        ),
    )


def _reason_text(
    candidate: DiscoveryModRecord,
    anchor: CandidateAnchor,
) -> str:
    parts: list[str] = []
    if anchor.shared_features:
        parts.append(
            "shared features " + ", ".join(anchor.shared_features)
        )
    if anchor.shared_categories:
        parts.append(
            "shared categories " + ", ".join(anchor.shared_categories)
        )
    evidence = "; ".join(parts)
    return (
        f"Because you use {anchor.installed.mod.name}: "
        f"{candidate.name} has {evidence}."
    )


def _latest_release_by_mod(
    releases: list[ReleaseRecord],
) -> dict[uuid.UUID, ReleaseRecord]:
    latest: dict[uuid.UUID, ReleaseRecord] = {}
    for release in releases:
        previous = latest.get(release.mod_id)
        if previous is None or _release_order_key(release) > _release_order_key(previous):
            latest[release.mod_id] = release
    return latest


def _release_order_key(
    release: ReleaseRecord,
) -> tuple[datetime, tuple[tuple[int, int | str], ...], str]:
    return (
        _utc(release.released_at),
        _release_version_key(release.version),
        str(release.id),
    )


def _release_version_key(value: str | None) -> tuple[tuple[int, int | str], ...]:
    if not value:
        return ()
    tokens = re.findall(r"\d+|[a-z]+", value.lower())
    return tuple(
        (1, int(token)) if token.isdigit() else (0, token)
        for token in tokens
    )


def _utc(value: datetime | None) -> datetime:
    if value is None:
        return datetime.min.replace(tzinfo=timezone.utc)
    if value.tzinfo is None:
        return value.replace(tzinfo=timezone.utc)
    return value.astimezone(timezone.utc)


def _rule_targets(
    rule: RelationshipRuleRecord,
    record: InstalledReleaseRecord,
) -> bool:
    target_matches = (
        rule.target_mod_id is not None
        and record.mod_id == rule.target_mod_id
    ) or (
        rule.target_source_kind is not None
        and rule.target_source_external_id is not None
        and record.source_kind == rule.target_source_kind
        and record.source_external_id == rule.target_source_external_id
    )
    if not target_matches:
        return False

    return _version_satisfies(
        record.version,
        rule.min_version,
        rule.max_version,
    )


def _version_satisfies(
    installed_version: str | None,
    minimum: str | None,
    maximum: str | None,
) -> bool:
    if minimum is None and maximum is None:
        return True
    if installed_version is None:
        return False

    if minimum is not None:
        comparison = _compare_versions(installed_version, minimum)
        if comparison is None or comparison < 0:
            return False
    if maximum is not None:
        comparison = _compare_versions(installed_version, maximum)
        if comparison is None or comparison > 0:
            return False
    return True


def _compare_versions(left: str | None, right: str | None) -> int | None:
    left_key = _numeric_version_key(left)
    right_key = _numeric_version_key(right)
    if not left_key or not right_key:
        return None
    width = max(len(left_key), len(right_key))
    left_padded = left_key + (0,) * (width - len(left_key))
    right_padded = right_key + (0,) * (width - len(right_key))
    if left_padded < right_padded:
        return -1
    if left_padded > right_padded:
        return 1
    return 0


def _numeric_version_key(value: str | None) -> tuple[int, ...]:
    if not value:
        return ()
    return tuple(int(part) for part in re.findall(r"\d+", value))
