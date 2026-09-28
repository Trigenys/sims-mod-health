from __future__ import annotations

from app.repositories.game_content_manifest import GameContentManifestRepository
from app.schemas.game_content import GameContentManifest


class GameContentManifestUnavailableError(LookupError):
    pass


class GameContentManifestService:
    def __init__(self, repository: GameContentManifestRepository) -> None:
        self._repository = repository

    def latest(self) -> GameContentManifest:
        document = self._repository.latest()
        if document is None:
            raise GameContentManifestUnavailableError(
                "No Game/DLC manifest is available in the registry."
            )

        payload = dict(document.payload_json)
        payload.update(
            {
                "schema_version": document.schema_version,
                "manifest_version": document.manifest_version,
                "source_identity": document.source_identity,
                "source_url": document.source_url,
                "retrieved_at": document.retrieved_at,
                "expires_at": document.expires_at,
                "checksum_sha256": document.checksum_sha256,
                "signature": document.signature,
            }
        )
        return GameContentManifest.model_validate(payload)
