from __future__ import annotations

from app.repositories.artifact_resolution import (
    ArtifactResolutionRecord,
    ArtifactResolutionRepository,
    FingerprintKey,
)
from app.schemas.resolution import (
    ArtifactMatch,
    ArtifactResolution,
    BatchResolveRequest,
    BatchResolveResponse,
    MatchEvidence,
)


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
        resolutions: list[ArtifactResolution] = []

        for probe in request.artifacts:
            by_artifact: dict[str, tuple[ArtifactResolutionRecord, list[MatchEvidence]]] = {}

            for key in probe_keys[probe.client_ref]:
                for record in exact.get(key, []):
                    artifact_key = str(record.artifact_id)
                    evidence = MatchEvidence(
                        kind=key.kind,
                        value=key.value,
                        algorithm_version=key.algorithm_version,
                    )

                    if artifact_key not in by_artifact:
                        by_artifact[artifact_key] = (record, [evidence])
                    else:
                        by_artifact[artifact_key][1].append(evidence)

            matches = [
                ArtifactMatch(
                    artifact_id=record.artifact_id,
                    release_id=record.release_id,
                    mod_id=record.mod_id,
                    creator_id=record.creator_id,
                    evidence=evidence,
                )
                for record, evidence in sorted(
                    by_artifact.values(),
                    key=lambda item: str(item[0].artifact_id),
                )
            ]

            resolutions.append(
                ArtifactResolution(client_ref=probe.client_ref, matches=matches)
            )

        return BatchResolveResponse(artifacts=resolutions)
