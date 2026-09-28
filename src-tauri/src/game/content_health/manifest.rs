use crate::registry::RegistryGameContentManifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManifestFingerprint {
    pub(crate) relative_path: String,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameBuildMetadata {
    pub(crate) version: String,
    pub(crate) released_at: Option<String>,
    pub(crate) fingerprints: Vec<ManifestFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackMetadata {
    pub(crate) code: String,
    pub(crate) pack_kind: String,
    pub(crate) min_game_version: Option<String>,
    pub(crate) released_at: Option<String>,
    pub(crate) expected_fingerprints: Vec<ManifestFingerprint>,
    pub(crate) evidence_source: String,
    pub(crate) evidence_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManifestProvenance {
    pub(crate) manifest_version: String,
    pub(crate) source_identity: String,
    pub(crate) source_url: Option<String>,
    pub(crate) retrieved_at: String,
    pub(crate) expires_at: Option<String>,
    pub(crate) checksum_sha256: Option<String>,
    pub(crate) signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameContentManifest {
    pub(crate) latest_game_build: String,
    pub(crate) game_builds: Vec<GameBuildMetadata>,
    pub(crate) packs: Vec<PackMetadata>,
    pub(crate) provenance: ManifestProvenance,
}

pub(crate) fn adapt_registry_manifest(
    wire: RegistryGameContentManifest,
) -> Result<GameContentManifest, String> {
    if wire.schema_version != 1 {
        return Err(format!(
            "unsupported Game/DLC manifest schema version {}",
            wire.schema_version
        ));
    }

    if wire.manifest_version.trim().is_empty()
        || wire.source_identity.trim().is_empty()
        || wire.latest_game_build.trim().is_empty()
    {
        return Err("Game/DLC manifest identity fields must not be empty".to_string());
    }

    let game_builds = wire
        .game_builds
        .into_iter()
        .map(|entry| GameBuildMetadata {
            version: entry.version,
            released_at: entry.released_at,
            fingerprints: entry
                .fingerprints
                .into_iter()
                .map(|fingerprint| ManifestFingerprint {
                    relative_path: fingerprint.relative_path.replace('\\', "/"),
                    sha256: fingerprint.sha256.to_ascii_lowercase(),
                })
                .collect(),
        })
        .collect::<Vec<_>>();

    if !game_builds
        .iter()
        .any(|entry| entry.version == wire.latest_game_build)
    {
        return Err("latest game build is absent from manifest build evidence".to_string());
    }

    let packs = wire
        .packs
        .into_iter()
        .map(|entry| PackMetadata {
            code: entry.code.to_ascii_uppercase(),
            pack_kind: entry.pack_kind,
            min_game_version: entry.min_game_version,
            released_at: entry.released_at,
            expected_fingerprints: entry
                .expected_fingerprints
                .into_iter()
                .map(|fingerprint| ManifestFingerprint {
                    relative_path: fingerprint.relative_path.replace('\\', "/"),
                    sha256: fingerprint.sha256.to_ascii_lowercase(),
                })
                .collect(),
            evidence_source: entry.evidence_source,
            evidence_url: entry.evidence_url,
        })
        .collect();

    Ok(GameContentManifest {
        latest_game_build: wire.latest_game_build,
        game_builds,
        packs,
        provenance: ManifestProvenance {
            manifest_version: wire.manifest_version,
            source_identity: wire.source_identity,
            source_url: wire.source_url,
            retrieved_at: wire.retrieved_at,
            expires_at: wire.expires_at,
            checksum_sha256: wire.checksum_sha256,
            signature: wire.signature,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{
        RegistryGameBuildManifestEntry, RegistryManifestFingerprint,
    };

    fn wire() -> RegistryGameContentManifest {
        RegistryGameContentManifest {
            schema_version: 1,
            manifest_version: "m1".to_string(),
            source_identity: "registry-curated".to_string(),
            source_url: None,
            retrieved_at: "2026-09-28T08:00:00Z".to_string(),
            expires_at: None,
            checksum_sha256: None,
            signature: None,
            latest_game_build: "1.128.90.1030".to_string(),
            game_builds: vec![RegistryGameBuildManifestEntry {
                version: "1.128.90.1030".to_string(),
                released_at: None,
                fingerprints: vec![RegistryManifestFingerprint {
                    relative_path: r"Game\Bin\Default.ini".to_string(),
                    sha256: "A".repeat(64),
                }],
            }],
            packs: Vec::new(),
        }
    }

    #[test]
    fn anti_corruption_layer_normalizes_manifest_evidence() {
        let manifest = adapt_registry_manifest(wire()).expect("manifest");

        assert_eq!(
            manifest.game_builds[0].fingerprints[0].relative_path,
            "Game/Bin/Default.ini"
        );
        assert_eq!(
            manifest.game_builds[0].fingerprints[0].sha256,
            "a".repeat(64)
        );
    }

    #[test]
    fn latest_build_must_have_evidence_entry() {
        let mut value = wire();
        value.latest_game_build = "1.999.0.0".to_string();

        assert!(adapt_registry_manifest(value).is_err());
    }
}
