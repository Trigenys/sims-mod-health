from __future__ import annotations

from datetime import datetime

from sqlalchemy import delete, select
from sqlalchemy.orm import Session

from app.domain import Artifact, Creator, Fingerprint, Mod, ModRelease, Source
from app.sources.github_releases.mapper import MappedProject


class GitHubReleasesIngestionRepository:
    def __init__(self, session: Session) -> None:
        self._session = session

    def upsert_project(
        self,
        project: MappedProject,
        *,
        retrieved_at: datetime,
    ) -> tuple[int, int]:
        source = self._get_or_create_source(project)
        creator = self._get_or_create_creator(project)
        mod = self._get_or_create_mod(project, creator)

        releases_upserted = 0
        artifacts_upserted = 0

        for mapped_release in project.releases:
            release = self._session.scalar(
                select(ModRelease).where(
                    ModRelease.source_id == source.id,
                    ModRelease.source_record_id == mapped_release.source_record_id,
                )
            )
            if release is None:
                release = ModRelease(
                    mod_id=mod.id,
                    source_id=source.id,
                    source_record_id=mapped_release.source_record_id,
                )
                self._session.add(release)

            release.mod_id = mod.id
            release.version = mapped_release.version
            release.released_at = mapped_release.released_at
            release.source_url = mapped_release.source_url
            release.retrieved_at = retrieved_at
            release.changelog = mapped_release.changelog
            self._session.flush()
            releases_upserted += 1

            for mapped_artifact in mapped_release.artifacts:
                artifact = self._session.scalar(
                    select(Artifact).where(
                        Artifact.source_id == source.id,
                        Artifact.source_record_id == mapped_artifact.source_record_id,
                    )
                )
                if artifact is None:
                    artifact = Artifact(
                        release_id=release.id,
                        source_id=source.id,
                        source_record_id=mapped_artifact.source_record_id,
                        artifact_kind=mapped_artifact.artifact_kind,
                    )
                    self._session.add(artifact)

                artifact.release_id = release.id
                artifact.artifact_kind = mapped_artifact.artifact_kind
                artifact.filename = mapped_artifact.filename
                artifact.size_bytes = mapped_artifact.size_bytes
                artifact.source_url = mapped_artifact.source_url
                artifact.retrieved_at = retrieved_at
                artifact.metadata_json = mapped_artifact.metadata
                self._session.flush()
                artifacts_upserted += 1

                self._replace_source_fingerprints(
                    artifact,
                    mapped_artifact.fingerprints,
                )

        source.name = project.mod_name
        source.base_url = project.source_url
        source.metadata_json = project.source_metadata
        mod.name = project.mod_name
        mod.description = project.summary
        mod.aliases = []
        creator.display_name = project.creator_name
        creator.aliases = []

        return releases_upserted, artifacts_upserted

    def _get_or_create_source(self, project: MappedProject) -> Source:
        source = self._session.scalar(
            select(Source).where(
                Source.kind == "github_releases",
                Source.external_id == project.source_external_id,
            )
        )
        if source is None:
            source = Source(
                kind="github_releases",
                external_id=project.source_external_id,
                name=project.mod_name,
                base_url=project.source_url,
                metadata_json=project.source_metadata,
            )
            self._session.add(source)
            self._session.flush()
        return source

    def _get_or_create_creator(self, project: MappedProject) -> Creator:
        creator = self._session.scalar(
            select(Creator).where(Creator.slug == project.creator_slug)
        )
        if creator is None:
            creator = Creator(
                slug=project.creator_slug,
                display_name=project.creator_name,
                aliases=[],
            )
            self._session.add(creator)
            self._session.flush()
        return creator

    def _get_or_create_mod(self, project: MappedProject, creator: Creator) -> Mod:
        mod = self._session.scalar(
            select(Mod).where(
                Mod.creator_id == creator.id,
                Mod.slug == project.mod_slug,
            )
        )
        if mod is None:
            mod = Mod(
                creator_id=creator.id,
                slug=project.mod_slug,
                name=project.mod_name,
                description=project.summary,
                aliases=[],
            )
            self._session.add(mod)
            self._session.flush()
        return mod

    def _replace_source_fingerprints(self, artifact: Artifact, fingerprints: tuple) -> None:
        self._session.execute(
            delete(Fingerprint).where(
                Fingerprint.artifact_id == artifact.id,
                Fingerprint.algorithm_version == "sha256-github-asset-digest-v1",
            )
        )

        for mapped in fingerprints:
            self._session.add(
                Fingerprint(
                    artifact_id=artifact.id,
                    kind=mapped.kind,
                    value=mapped.value,
                    algorithm_version=mapped.algorithm_version,
                )
            )
