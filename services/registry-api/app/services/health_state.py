from __future__ import annotations

import re
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import cast

from app.repositories.health_state import (
    CompatibilityEvidenceRecord,
    HealthStateRepository,
    ReleaseRecord,
)
from app.schemas.health import (
    CompatibilityEvidence,
    CompatibilityState,
    HealthAssessment,
    HealthEvaluateRequest,
    HealthEvaluateResponse,
    UpdateAssessment,
)


COMPATIBILITY_STATES = frozenset(
    {"compatible", "unknown", "potential_conflict", "broken", "abandoned"}
)


class UnknownReleaseError(ValueError):
    def __init__(self, release_ids: list[uuid.UUID]) -> None:
        self.release_ids = release_ids
        super().__init__(
            "Unknown installed release ids: "
            + ", ".join(str(item) for item in release_ids)
        )


@dataclass(frozen=True, slots=True)
class CompatibilityDecision:
    state: CompatibilityState
    disputed: bool
    reason: str
    evidence: tuple[CompatibilityEvidenceRecord, ...]


class HealthStateService:
    def __init__(self, repository: HealthStateRepository) -> None:
        self._repository = repository

    def evaluate(self, request: HealthEvaluateRequest) -> HealthEvaluateResponse:
        installed_ids = set(request.installed_release_ids)
        installed = self._repository.get_releases(installed_ids)
        missing = [
            release_id
            for release_id in request.installed_release_ids
            if release_id not in installed
        ]
        if missing:
            raise UnknownReleaseError(missing)

        mod_ids = {record.mod_id for record in installed.values()}
        all_releases = self._repository.list_releases_for_mods(mod_ids)
        releases_by_mod: dict[uuid.UUID, list[ReleaseRecord]] = {}
        for release in all_releases:
            releases_by_mod.setdefault(release.mod_id, []).append(release)

        all_release_ids = {release.id for release in all_releases}
        evidence = self._repository.list_compatibility_evidence(all_release_ids)
        evidence_by_release: dict[uuid.UUID, list[CompatibilityEvidenceRecord]] = {}
        for report in evidence:
            evidence_by_release.setdefault(report.release_id, []).append(report)

        items: list[HealthAssessment] = []
        for release_id in request.installed_release_ids:
            release = installed[release_id]
            decision = self._compatibility_for_release(
                release.id,
                request.patch_version,
                evidence_by_release,
            )
            update = self._update_for_release(
                release,
                releases_by_mod.get(release.mod_id, []),
                request.patch_version,
                evidence_by_release,
            )
            final_state, reason = self._final_state(decision, update)
            items.append(
                HealthAssessment(
                    release_id=release.id,
                    mod_id=release.mod_id,
                    state=final_state,
                    compatibility_state=decision.state,
                    disputed=decision.disputed,
                    reason=reason,
                    evidence=[
                        self._serialize_evidence(report)
                        for report in decision.evidence
                    ],
                    update=update,
                )
            )

        return HealthEvaluateResponse(
            patch_version=request.patch_version,
            platform=request.platform,
            items=items,
        )

    def _compatibility_for_release(
        self,
        release_id: uuid.UUID,
        patch_version: str,
        evidence_by_release: dict[
            uuid.UUID, list[CompatibilityEvidenceRecord]
        ],
    ) -> CompatibilityDecision:
        applicable: list[tuple[int, CompatibilityEvidenceRecord]] = []
        for report in evidence_by_release.get(release_id, []):
            specificity = self._scope_specificity(report, patch_version)
            if specificity > 0:
                applicable.append((specificity, report))

        if not applicable:
            return CompatibilityDecision(
                state="unknown",
                disputed=False,
                reason="no_current_patch_evidence",
                evidence=(),
            )

        max_specificity = max(item[0] for item in applicable)
        scoped = [
            report
            for specificity, report in applicable
            if specificity == max_specificity
        ]

        latest_by_source: dict[uuid.UUID, CompatibilityEvidenceRecord] = {}
        for report in scoped:
            previous = latest_by_source.get(report.source_id)
            if previous is None or report.retrieved_at > previous.retrieved_at:
                latest_by_source[report.source_id] = report

        current = tuple(
            sorted(
                latest_by_source.values(),
                key=lambda item: (
                    item.retrieved_at,
                    str(item.source_id),
                    str(item.report_id),
                ),
                reverse=True,
            )
        )
        statuses = {
            report.status
            for report in current
            if report.status in COMPATIBILITY_STATES
        }

        if not statuses:
            return CompatibilityDecision(
                state="unknown",
                disputed=False,
                reason="no_compatibility_status",
                evidence=current,
            )

        if len(statuses) > 1:
            return CompatibilityDecision(
                state="unknown",
                disputed=True,
                reason="conflicting_current_patch_evidence",
                evidence=current,
            )

        status = cast(CompatibilityState, next(iter(statuses)))
        return CompatibilityDecision(
            state=status,
            disputed=False,
            reason=(
                "exact_patch_evidence"
                if max_specificity == 2
                else "patch_range_evidence"
            ),
            evidence=current,
        )

    def _update_for_release(
        self,
        installed: ReleaseRecord,
        mod_releases: list[ReleaseRecord],
        patch_version: str,
        evidence_by_release: dict[
            uuid.UUID, list[CompatibilityEvidenceRecord]
        ],
    ) -> UpdateAssessment:
        newer = [
            release
            for release in mod_releases
            if release.id != installed.id and _is_newer_release(release, installed)
        ]
        if not newer:
            return UpdateAssessment(available=False)

        target = max(newer, key=_release_order_key)
        target_decision = self._compatibility_for_release(
            target.id,
            patch_version,
            evidence_by_release,
        )
        return UpdateAssessment(
            available=True,
            target_release_id=target.id,
            target_version=target.version,
            target_compatibility_state=target_decision.state,
            target_disputed=target_decision.disputed,
        )

    def _final_state(
        self,
        decision: CompatibilityDecision,
        update: UpdateAssessment,
    ) -> tuple[str, str]:
        if decision.state in {"broken", "abandoned", "potential_conflict"}:
            return decision.state, decision.reason
        if update.available:
            return "update_available", "newer_release_available"
        return decision.state, decision.reason

    def _scope_specificity(
        self,
        report: CompatibilityEvidenceRecord,
        patch_version: str,
    ) -> int:
        if report.patch_id is not None:
            return 2 if _versions_equal(report.patch_version, patch_version) else 0

        if not _version_key(patch_version):
            return 0

        if report.patch_min_version:
            comparison = _compare_versions(patch_version, report.patch_min_version)
            if comparison is None or comparison < 0:
                return 0
        if report.patch_max_version:
            comparison = _compare_versions(patch_version, report.patch_max_version)
            if comparison is None or comparison > 0:
                return 0
        return 1

    def _serialize_evidence(
        self,
        report: CompatibilityEvidenceRecord,
    ) -> CompatibilityEvidence:
        exact = report.patch_id is not None
        return CompatibilityEvidence(
            report_id=report.report_id,
            source_id=report.source_id,
            source_url=report.source_url,
            source_record_id=report.source_record_id,
            status=report.status,
            retrieved_at=report.retrieved_at,
            notes=report.notes,
            scope="exact_patch" if exact else "patch_range",
            patch_version=report.patch_version if exact else None,
            patch_min_version=report.patch_min_version if not exact else None,
            patch_max_version=report.patch_max_version if not exact else None,
        )


def _version_key(value: str | None) -> tuple[int, ...]:
    if not value:
        return ()
    parts = re.findall(r"\d+", value)
    if not parts:
        return ()
    return tuple(int(part) for part in parts)


def _compare_versions(left: str | None, right: str | None) -> int | None:
    left_key = _version_key(left)
    right_key = _version_key(right)
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


def _versions_equal(left: str | None, right: str | None) -> bool:
    return _compare_versions(left, right) == 0


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


def _release_order_key(
    release: ReleaseRecord,
) -> tuple[datetime, tuple[tuple[int, int | str], ...], str]:
    return (
        _utc(release.released_at),
        _release_version_key(release.version),
        str(release.id),
    )


def _is_newer_release(candidate: ReleaseRecord, installed: ReleaseRecord) -> bool:
    if candidate.released_at is not None and installed.released_at is not None:
        candidate_date = _utc(candidate.released_at)
        installed_date = _utc(installed.released_at)
        if candidate_date != installed_date:
            return candidate_date > installed_date

    candidate_version = _release_version_key(candidate.version)
    installed_version = _release_version_key(installed.version)
    if candidate_version and installed_version and candidate_version != installed_version:
        return candidate_version > installed_version

    return False
