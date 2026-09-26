from __future__ import annotations

from datetime import datetime, timezone

from app.sources.curseforge.mapper import CurseForgeMapper
from app.sources.curseforge.models import CurseForgeFile, CurseForgeMod


def test_mapper_preserves_source_identity_hashes_versions_dependencies() -> None:
    mod = CurseForgeMod.model_validate(
        {
            "id": 42,
            "gameId": 99,
            "name": "Example Mod",
            "slug": "example-mod",
            "summary": "Summary",
            "authors": [
                {"id": 7, "name": "Primary", "url": "https://example.test/primary"},
                {"id": 8, "name": "Coauthor", "url": "https://example.test/coauthor"},
            ],
            "links": {"websiteUrl": "https://example.test/mod"},
            "isAvailable": True,
        }
    )
    file = CurseForgeFile.model_validate(
        {
            "id": 1001,
            "gameId": 99,
            "modId": 42,
            "isAvailable": True,
            "displayName": "2.4.1",
            "fileName": "ExampleMod.ts4script",
            "releaseType": 1,
            "fileStatus": 4,
            "hashes": [
                {"value": "ABCDEF", "algo": 1},
                {"value": "123456", "algo": 2},
            ],
            "fileDate": "2026-09-01T00:00:00Z",
            "fileLength": 1234,
            "fileSizeOnDisk": 1500,
            "downloadUrl": "https://example.test/file",
            "gameVersions": ["1.116.240"],
            "dependencies": [
                {"modId": 77, "relationType": 3},
                {"modId": 88, "relationType": 5},
            ],
            "fileFingerprint": 987654321,
            "modules": [{"name": "example", "fingerprint": 123}],
        }
    )

    mapped = CurseForgeMapper().map_project(
        mod,
        [file],
        retrieved_at=datetime(2026, 9, 26, tzinfo=timezone.utc),
        changelogs={1001: "Fixed things"},
    )

    assert mapped.source_external_id == "42"
    assert mapped.creator_slug == "curseforge-author-7"
    assert mapped.creator_aliases == ("Coauthor",)
    assert mapped.files[0].artifact_kind == "ts4script"
    assert mapped.files[0].changelog == "Fixed things"
    assert [(fp.kind, fp.value) for fp in mapped.files[0].fingerprints] == [
        ("curseforge", "987654321"),
        ("sha1", "abcdef"),
        ("md5", "123456"),
    ]
    metadata = mapped.files[0].metadata["curseforge"]
    assert metadata["game_versions"] == ["1.116.240"]
    assert metadata["dependencies"] == [
        {"mod_id": 77, "relation_type": 3, "relation": "required_dependency"},
        {"mod_id": 88, "relation_type": 5, "relation": "incompatible"},
    ]
