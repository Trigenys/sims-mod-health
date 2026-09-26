from __future__ import annotations

from datetime import datetime

from sqlalchemy import delete, select
from sqlalchemy.orm import Session

from app.domain import (
    Artifact,
    ConflictRule,
    Creator,
    DependencyRule,
    Fingerprint,
    Mod,
    ModRelease,
    Source,
)
from app.services.discovery_taxonomy import infer_taxonomy
from app.sources.curseforge.mapper import MappedFile, MappedProject


class CurseForgeIngestionRepository:
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

        for mapped_file in project.files:
            release = self._session.scalar(
                select(ModRelease).where(
                    ModRelease.source_id == source.id,
                    ModRelease.source_record_id == mapped_file.source_record_id,
                )
            )
            if release is None:
                release = ModRelease(
                    mod_id=mod.id,
                    source_id=source.id,
                    source_record_id=mapped_file.source_record_id,
                )
                self._session.add(release)

            release.mod_id = mod.id
            release.version = mapped_file.display_name
            release.released_at = mapped_file.released_at
            release.source_url = mapped_file.source_url
            release.retrieved_at = retrieved_at
            release.changelog = mapped_file.changelog
            self._session.flush()
            releases_upserted += 1

            artifact = self._session.scalar(
                select(Artifact).where(
                    Artifact.source_id == source.id,
                    Artifact.source_record_id == mapped_file.source_record_id,
                )
            )
            if artifact is None:
                artifact = Artifact(
                    release_id=release.id,
                    source_id=source.id,
                    source_record_id=mapped_file.source_record_id,
                    artifact_kind=mapped_file.artifact_kind,
                )
                self._session.add(artifact)

            artifact.release_id = release.id
            artifact.artifact_kind = mapped_file.artifact_kind
            artifact.filename = mapped_file.filename
            artifact.size_bytes = mapped_file.size_bytes
            artifact.source_url = mapped_file.source_url
            artifact.retrieved_at = retrieved_at
            artifact.metadata_json = mapped_file.metadata
            self._session.flush()
            artifacts_upserted += 1

            self._replace_source_fingerprints(artifact, mapped_file.fingerprints)
            self._replace_relationship_rules(
                source,
                release,
                mapped_file,
                retrieved_at=retrieved_at,
            )

        source.name = project.mod_name
        source.base_url = project.source_url
        source.metadata_json = project.source_metadata
        mod.name = project.mod_name
        mod.description = project.summary
        mod.aliases = []
        categories, features = infer_taxonomy(
            name=project.mod_name,
            summary=project.summary,
            slug=project.mod_slug,
        )
        if not mod.categories:
            mod.categories = categories
        if not mod.features:
            mod.features = features
        creator.display_name = project.creator_name
        creator.aliases = list(project.creator_aliases)

        return releases_upserted, artifacts_upserted

    def _get_or_create_source(self, project: MappedProject) -> Source:
        source = self._session.scalar(
            select(Source).where(
                Source.kind == "curseforge",
                Source.external_id == project.source_external_id,
            )
        )
        if source is None:
            source = Source(
                kind="curseforge",
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
                aliases=list(project.creator_aliases),
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
        source_kinds = {"curseforge", "sha1", "md5"}
        self._session.execute(
            delete(Fingerprint).where(
                Fingerprint.artifact_id == artifact.id,
                Fingerprint.kind.in_(source_kinds),
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

    def _replace_relationship_rules(
        self,
        source: Source,
        release: ModRelease,
        mapped_file: MappedFile,
        *,
        retrieved_at: datetime,
    ) -> None:
        self._session.execute(
            delete(DependencyRule).where(
                DependencyRule.release_id == release.id,
                DependencyRule.source_id == source.id,
            )
        )
        self._session.execute(
            delete(ConflictRule).where(
                ConflictRule.release_id == release.id,
                ConflictRule.source_id == source.id,
            )
        )

        for relationship in mapped_file.relationships:
            common = {
                "release_id": release.id,
                "target_mod_id": None,
                "target_source_kind": "curseforge",
                "target_source_external_id": relationship.target_source_external_id,
                "min_version": None,
                "max_version": None,
                "source_id": source.id,
                "source_url": source.base_url,
                "source_record_id": (
                    f"{mapped_file.source_record_id}:"
                    f"{relationship.target_source_external_id}:"
                    f"{relationship.relation}"
                ),
                "retrieved_at": retrieved_at,
                "notes": (
                    "Normalized from the CurseForge file dependency relation "
                    f"{relationship.relation}."
                ),
            }
            if relationship.relation == "required_dependency":
                self._session.add(DependencyRule(**common))
            elif relationship.relation == "incompatible":
                self._session.add(ConflictRule(**common))
