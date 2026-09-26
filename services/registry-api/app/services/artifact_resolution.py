from __future__ import annotations

import re
import unicodedata
from dataclasses import dataclass, field
from difflib import SequenceMatcher
from pathlib import Path

from app.repositories.artifact_resolution import (
    ArtifactCandidateRecord,
    ArtifactResolutionRecord,
    ArtifactResolutionRepository,
    FingerprintKey,
)
from app.schemas.resolution import (
    ArtifactMatch,
    ArtifactProbe,
    ArtifactResolution,
    BatchResolveRequest,
    BatchResolveResponse,
    EmbeddedMetadataEvidence,
    ExactFingerprintEvidence,
    FuzzyNameEvidence,
    MatchEvidence,
    NormalizedFilenameEvidence,
    SizeEvidence,
    StructuralFingerprintEvidence,
)


MIN_CANDIDATE_SCORE = 0.72
MIN_RESOLVED_SCORE = 0.78
AMBIGUITY_MARGIN = 0.06
FUZZY_THRESHOLD = 0.76


@dataclass(slots=True)
class CandidateAssessment:
    candidate: ArtifactCandidateRecord
    score: float = 0.0
    stage: str = "fuzzy_candidate"
    evidence: list[MatchEvidence] = field(default_factory=list)


class ArtifactResolutionService:
    def __init__(self, repository: ArtifactResolutionRepository) -> None:
        self._repository = repository

    def resolve(self, request: BatchResolveRequest) -> BatchResolveResponse:
        probe_keys: dict[str, list[FingerprintKey]] = {}
        all_keys: set[FingerprintKey] = set()

        for probe in request.artifacts:
            keys = [
                FingerprintKey(
                    fingerprint.kind,
                    fingerprint.value,
                    fingerprint.algorithm_version,
                )
                for fingerprint in probe.fingerprints
            ]
            probe_keys[probe.client_ref] = keys
            all_keys.update(keys)

        exact = self._repository.find_exact(all_keys)
        structural = self._repository.find_structural(all_keys)
        candidate_cache: dict[str | None, list[ArtifactCandidateRecord]] = {}

        resolutions = [
            self._resolve_probe(
                probe,
                probe_keys[probe.client_ref],
                exact,
                structural,
                candidate_cache,
            )
            for probe in request.artifacts
        ]

        return BatchResolveResponse(artifacts=resolutions)

    def _resolve_probe(
        self,
        probe: ArtifactProbe,
        keys: list[FingerprintKey],
        exact: dict[FingerprintKey, list[ArtifactResolutionRecord]],
        structural: dict[FingerprintKey, list[ArtifactResolutionRecord]],
        candidate_cache: dict[str | None, list[ArtifactCandidateRecord]],
    ) -> ArtifactResolution:
        exact_matches = self._exact_matches(keys, exact)
        if exact_matches:
            status = "resolved" if len(exact_matches) == 1 else "ambiguous"
            selected = exact_matches[0].artifact_id if status == "resolved" else None
            return ArtifactResolution(
                client_ref=probe.client_ref,
                status=status,
                selected_artifact_id=selected,
                matches=exact_matches,
            )

        cache_key = probe.artifact_kind if probe.artifact_kind != "unknown" else None
        if cache_key not in candidate_cache:
            candidate_cache[cache_key] = self._repository.list_candidates(cache_key)

        structural_by_artifact = self._structural_evidence(keys, structural)
        assessments = [
            self._assess_candidate(
                probe,
                candidate,
                structural_by_artifact.get(str(candidate.artifact_id), []),
            )
            for candidate in candidate_cache[cache_key]
        ]
        assessments = [
            item for item in assessments if item.score >= MIN_CANDIDATE_SCORE
        ]
        assessments.sort(
            key=lambda item: (-item.score, str(item.candidate.artifact_id))
        )

        matches = [self._to_probabilistic_match(item) for item in assessments]
        if not matches or matches[0].score < MIN_RESOLVED_SCORE:
            return ArtifactResolution(
                client_ref=probe.client_ref,
                status="unresolved",
                selected_artifact_id=None,
                matches=matches[:5],
            )

        if len(matches) > 1 and matches[0].score - matches[1].score < AMBIGUITY_MARGIN:
            return ArtifactResolution(
                client_ref=probe.client_ref,
                status="ambiguous",
                selected_artifact_id=None,
                matches=matches[:5],
            )

        return ArtifactResolution(
            client_ref=probe.client_ref,
            status="resolved",
            selected_artifact_id=matches[0].artifact_id,
            matches=matches[:5],
        )

    def _exact_matches(
        self,
        keys: list[FingerprintKey],
        exact: dict[FingerprintKey, list[ArtifactResolutionRecord]],
    ) -> list[ArtifactMatch]:
        grouped: dict[str, tuple[ArtifactResolutionRecord, list[MatchEvidence]]] = {}

        for key in keys:
            for record in exact.get(key, []):
                artifact_key = str(record.artifact_id)
                evidence = ExactFingerprintEvidence(
                    kind=key.kind,
                    value=key.value,
                    algorithm_version=key.algorithm_version,
                )
                if artifact_key not in grouped:
                    grouped[artifact_key] = (record, [evidence])
                else:
                    grouped[artifact_key][1].append(evidence)

        return [
            ArtifactMatch(
                artifact_id=record.artifact_id,
                release_id=record.release_id,
                mod_id=record.mod_id,
                creator_id=record.creator_id,
                stage="exact_fingerprint",
                confidence="exact",
                score=1.0,
                deterministic=True,
                evidence=evidence,
            )
            for record, evidence in sorted(
                grouped.values(),
                key=lambda item: str(item[0].artifact_id),
            )
        ]

    def _structural_evidence(
        self,
        keys: list[FingerprintKey],
        structural: dict[FingerprintKey, list[ArtifactResolutionRecord]],
    ) -> dict[str, list[StructuralFingerprintEvidence]]:
        grouped: dict[str, list[StructuralFingerprintEvidence]] = {}
        for key in keys:
            for record in structural.get(key, []):
                grouped.setdefault(str(record.artifact_id), []).append(
                    StructuralFingerprintEvidence(
                        kind=key.kind,
                        value=key.value,
                        algorithm_version=key.algorithm_version,
                    )
                )
        return grouped

    def _assess_candidate(
        self,
        probe: ArtifactProbe,
        candidate: ArtifactCandidateRecord,
        structural_evidence: list[StructuralFingerprintEvidence],
    ) -> CandidateAssessment:
        assessment = CandidateAssessment(candidate=candidate)
        hints = probe.identity_hints

        mod_exact = False
        creator_exact = False
        version_exact = False

        if hints and hints.mod_name:
            mod_exact = _matches_any(
                hints.mod_name,
                [candidate.mod_name, candidate.mod_slug, *candidate.mod_aliases],
            )
            if mod_exact:
                assessment.evidence.append(
                    EmbeddedMetadataEvidence(
                        field="mod_name",
                        value=hints.mod_name,
                    )
                )

        if hints and hints.creator:
            creator_exact = _matches_any(
                hints.creator,
                [
                    candidate.creator_name,
                    candidate.creator_slug,
                    *candidate.creator_aliases,
                ],
            )
            if creator_exact:
                assessment.evidence.append(
                    EmbeddedMetadataEvidence(
                        field="creator",
                        value=hints.creator,
                    )
                )

        if hints and hints.version and candidate.release_version:
            version_exact = _normalize(hints.version) == _normalize(
                candidate.release_version
            )
            if version_exact:
                assessment.evidence.append(
                    EmbeddedMetadataEvidence(
                        field="version",
                        value=hints.version,
                    )
                )

        if mod_exact and creator_exact:
            assessment.score = 0.95
            assessment.stage = "embedded_metadata"
        elif mod_exact:
            assessment.score = 0.89
            assessment.stage = "embedded_metadata"
        elif creator_exact and version_exact:
            assessment.score = 0.82
            assessment.stage = "embedded_metadata"

        local_filename = probe.filename
        registry_filename = candidate.filename
        filename_exact = False
        fuzzy_similarity = 0.0

        if local_filename and registry_filename:
            local_key = _filename_key(local_filename)
            registry_key = _filename_key(registry_filename)
            filename_exact = bool(local_key and local_key == registry_key)
            if filename_exact:
                assessment.evidence.append(
                    NormalizedFilenameEvidence(
                        local_filename=local_filename,
                        registry_filename=registry_filename,
                    )
                )
                if assessment.score < 0.86:
                    assessment.score = 0.86
                    assessment.stage = "normalized_name"
            elif local_key and registry_key:
                fuzzy_similarity = max(
                    SequenceMatcher(None, local_key, registry_key).ratio(),
                    SequenceMatcher(
                        None,
                        local_key,
                        _normalize(candidate.mod_name),
                    ).ratio(),
                    SequenceMatcher(
                        None,
                        local_key,
                        _normalize(candidate.mod_slug),
                    ).ratio(),
                )
                if fuzzy_similarity >= FUZZY_THRESHOLD:
                    assessment.evidence.append(
                        FuzzyNameEvidence(
                            local_value=local_filename,
                            registry_value=registry_filename,
                            similarity=round(fuzzy_similarity, 4),
                        )
                    )
                    fuzzy_score = 0.61 + (0.26 * fuzzy_similarity)
                    if assessment.score < fuzzy_score:
                        assessment.score = fuzzy_score
                        assessment.stage = "fuzzy_candidate"

        if structural_evidence:
            assessment.evidence.extend(structural_evidence)
            if assessment.score < 0.87:
                assessment.score = 0.87
                assessment.stage = "resource_signature"
            else:
                assessment.score = min(0.99, assessment.score + 0.04)

        if version_exact and assessment.score > 0:
            assessment.score = min(0.99, assessment.score + 0.03)
        if creator_exact and assessment.score > 0 and assessment.stage != "embedded_metadata":
            assessment.score = min(0.99, assessment.score + 0.03)
        if (
            probe.size_bytes is not None
            and candidate.size_bytes is not None
            and probe.size_bytes == candidate.size_bytes
            and assessment.score > 0
        ):
            assessment.evidence.append(SizeEvidence(size_bytes=probe.size_bytes))
            assessment.score = min(0.99, assessment.score + 0.02)

        return assessment

    def _to_probabilistic_match(
        self,
        assessment: CandidateAssessment,
    ) -> ArtifactMatch:
        if assessment.score >= 0.88:
            confidence = "high"
        elif assessment.score >= 0.78:
            confidence = "medium"
        else:
            confidence = "low"

        candidate = assessment.candidate
        return ArtifactMatch(
            artifact_id=candidate.artifact_id,
            release_id=candidate.release_id,
            mod_id=candidate.mod_id,
            creator_id=candidate.creator_id,
            stage=assessment.stage,
            confidence=confidence,
            score=round(assessment.score, 4),
            deterministic=False,
            evidence=assessment.evidence,
        )


def _normalize(value: str) -> str:
    normalized = unicodedata.normalize("NFKD", value)
    ascii_value = "".join(
        character
        for character in normalized
        if not unicodedata.combining(character)
    )
    return re.sub(r"[^a-z0-9]+", "", ascii_value.lower())


def _filename_key(filename: str) -> str:
    stem = Path(filename).stem
    without_version = re.sub(
        r"(?:^|[\s._-])v?\d+(?:[._-]\d+){0,3}(?:$|[\s._-])",
        " ",
        stem,
        flags=re.IGNORECASE,
    )
    normalized = _normalize(without_version)
    return normalized or _normalize(stem)


def _matches_any(value: str, candidates: list[str]) -> bool:
    normalized = _normalize(value)
    return bool(normalized) and normalized in {
        _normalize(candidate) for candidate in candidates if candidate
    }
