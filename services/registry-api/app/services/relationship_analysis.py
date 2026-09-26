from __future__ import annotations

import re
import uuid
from collections import defaultdict
from datetime import datetime, timezone

from app.repositories.relationship_analysis import (
    InstalledReleaseRecord,
    RelationshipAnalysisRepository,
    RelationshipRuleRecord,
)
from app.schemas.relationships import (
    DependencyCycle,
    DependencyFinding,
    InstallationRelationshipRequest,
    InstallationRelationshipResponse,
    KnownIncompatibilityFinding,
    ReverseDependencyUsage,
    RuleProvenance,
    RuleTarget,
)


class UnknownInstalledReleaseError(ValueError):
    def __init__(self, release_ids: list[uuid.UUID]) -> None:
        self.release_ids = release_ids
        super().__init__(
            "Unknown installed release ids: "
            + ", ".join(str(item) for item in release_ids)
        )


class RelationshipAnalysisService:
    def __init__(self, repository: RelationshipAnalysisRepository) -> None:
        self._repository = repository

    def analyze(
        self,
        request: InstallationRelationshipRequest,
    ) -> InstallationRelationshipResponse:
        requested = set(request.installed_release_ids)
        installed = self._repository.get_installed_releases(requested)
        missing_ids = [
            release_id
            for release_id in request.installed_release_ids
            if release_id not in installed
        ]
        if missing_ids:
            raise UnknownInstalledReleaseError(missing_ids)

        dependency_rules = self._repository.list_dependency_rules(requested)
        conflict_rules = self._repository.list_conflict_rules(requested)

        dependency_findings: list[DependencyFinding] = []
        known_incompatibilities: list[KnownIncompatibilityFinding] = []
        reverse_usage: dict[uuid.UUID, set[uuid.UUID]] = defaultdict(set)
        graph: dict[uuid.UUID, set[uuid.UUID]] = defaultdict(set)

        records = list(installed.values())

        for rule in dependency_rules:
            candidates = self._target_candidates(rule, records)
            if not candidates:
                dependency_findings.append(
                    DependencyFinding(
                        rule_id=rule.rule_id,
                        required_by_release_id=rule.release_id,
                        status="missing",
                        target=self._target(rule),
                        installed_target_release_id=None,
                        installed_target_version=None,
                        action="install_dependency",
                        provenance=self._provenance(rule),
                    )
                )
                continue

            target = max(candidates, key=_release_order_key)
            reverse_usage[target.release_id].add(rule.release_id)
            graph[rule.release_id].add(target.release_id)

            version_status = _dependency_version_status(
                target.version,
                rule.min_version,
                rule.max_version,
            )
            if version_status is None:
                continue

            status, action = version_status
            dependency_findings.append(
                DependencyFinding(
                    rule_id=rule.rule_id,
                    required_by_release_id=rule.release_id,
                    status=status,
                    target=self._target(rule),
                    installed_target_release_id=target.release_id,
                    installed_target_version=target.version,
                    action=action,
                    provenance=self._provenance(rule),
                )
            )

        seen_conflicts: set[tuple[uuid.UUID, uuid.UUID, uuid.UUID]] = set()
        for rule in conflict_rules:
            for target in self._target_candidates(rule, records):
                if not _version_satisfies(
                    target.version,
                    rule.min_version,
                    rule.max_version,
                ):
                    continue

                left, right = sorted(
                    (rule.release_id, target.release_id),
                    key=str,
                )
                identity = (rule.rule_id, left, right)
                if identity in seen_conflicts:
                    continue
                seen_conflicts.add(identity)

                known_incompatibilities.append(
                    KnownIncompatibilityFinding(
                        rule_id=rule.rule_id,
                        left_release_id=rule.release_id,
                        right_release_id=target.release_id,
                        right_version=target.version,
                        target=self._target(rule),
                        provenance=self._provenance(rule),
                    )
                )

        reverse = [
            ReverseDependencyUsage(
                dependency_release_id=release_id,
                used_by_count=len(users),
                used_by_release_ids=sorted(users, key=str),
            )
            for release_id, users in sorted(
                reverse_usage.items(),
                key=lambda item: str(item[0]),
            )
        ]

        cycles = [
            DependencyCycle(release_ids=component)
            for component in _strongly_connected_cycles(
                graph,
                set(installed),
            )
        ]

        dependency_findings.sort(
            key=lambda finding: (
                str(finding.required_by_release_id),
                str(finding.rule_id),
            )
        )
        known_incompatibilities.sort(
            key=lambda finding: (
                str(finding.left_release_id),
                str(finding.right_release_id),
                str(finding.rule_id),
            )
        )

        return InstallationRelationshipResponse(
            dependency_findings=dependency_findings,
            known_incompatibilities=known_incompatibilities,
            reverse_usage=reverse,
            cycles=cycles,
        )

    def _target_candidates(
        self,
        rule: RelationshipRuleRecord,
        installed: list[InstalledReleaseRecord],
    ) -> list[InstalledReleaseRecord]:
        matches: list[InstalledReleaseRecord] = []
        for release in installed:
            by_mod = (
                rule.target_mod_id is not None
                and release.mod_id == rule.target_mod_id
            )
            by_source = (
                rule.target_source_kind is not None
                and rule.target_source_external_id is not None
                and release.source_kind == rule.target_source_kind
                and release.source_external_id == rule.target_source_external_id
            )
            if by_mod or by_source:
                matches.append(release)
        return matches

    def _target(self, rule: RelationshipRuleRecord) -> RuleTarget:
        return RuleTarget(
            mod_id=rule.target_mod_id,
            source_kind=rule.target_source_kind,
            source_external_id=rule.target_source_external_id,
            min_version=rule.min_version,
            max_version=rule.max_version,
        )

    def _provenance(self, rule: RelationshipRuleRecord) -> RuleProvenance:
        return RuleProvenance(
            source_id=rule.source_id,
            source_url=rule.source_url,
            source_record_id=rule.source_record_id,
            retrieved_at=rule.retrieved_at,
            notes=rule.notes,
        )


def _dependency_version_status(
    installed_version: str | None,
    minimum: str | None,
    maximum: str | None,
) -> tuple[str, str] | None:
    if minimum is None and maximum is None:
        return None

    if installed_version is None:
        return ("version_mismatch", "review_dependency_version")

    if minimum is not None:
        comparison = _compare_versions(installed_version, minimum)
        if comparison is None:
            return ("version_mismatch", "review_dependency_version")
        if comparison < 0:
            return ("outdated", "update_dependency")

    if maximum is not None:
        comparison = _compare_versions(installed_version, maximum)
        if comparison is None or comparison > 0:
            return ("version_mismatch", "review_dependency_version")

    return None


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


def _version_key(value: str | None) -> tuple[int, ...]:
    if not value:
        return ()
    numbers = tuple(int(part) for part in re.findall(r"\d+", value))
    return numbers


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


def _utc(value: datetime | None) -> datetime:
    if value is None:
        return datetime.min.replace(tzinfo=timezone.utc)
    if value.tzinfo is None:
        return value.replace(tzinfo=timezone.utc)
    return value.astimezone(timezone.utc)


def _release_order_key(
    release: InstalledReleaseRecord,
) -> tuple[datetime, tuple[int, ...], str]:
    return (
        _utc(release.released_at),
        _version_key(release.version),
        str(release.release_id),
    )


def _strongly_connected_cycles(
    graph: dict[uuid.UUID, set[uuid.UUID]],
    nodes: set[uuid.UUID],
) -> list[list[uuid.UUID]]:
    index = 0
    indices: dict[uuid.UUID, int] = {}
    lowlinks: dict[uuid.UUID, int] = {}
    stack: list[uuid.UUID] = []
    on_stack: set[uuid.UUID] = set()
    components: list[list[uuid.UUID]] = []

    def visit(node: uuid.UUID) -> None:
        nonlocal index
        indices[node] = index
        lowlinks[node] = index
        index += 1
        stack.append(node)
        on_stack.add(node)

        for neighbor in sorted(graph.get(node, set()), key=str):
            if neighbor not in indices:
                visit(neighbor)
                lowlinks[node] = min(lowlinks[node], lowlinks[neighbor])
            elif neighbor in on_stack:
                lowlinks[node] = min(lowlinks[node], indices[neighbor])

        if lowlinks[node] != indices[node]:
            return

        component: list[uuid.UUID] = []
        while stack:
            member = stack.pop()
            on_stack.remove(member)
            component.append(member)
            if member == node:
                break

        has_self_loop = (
            len(component) == 1
            and component[0] in graph.get(component[0], set())
        )
        if len(component) > 1 or has_self_loop:
            components.append(sorted(component, key=str))

    for node in sorted(nodes, key=str):
        if node not in indices:
            visit(node)

    components.sort(key=lambda component: tuple(str(item) for item in component))
    return components
