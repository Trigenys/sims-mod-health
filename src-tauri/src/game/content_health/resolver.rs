use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap},
};

use serde::Serialize;

use super::{
    manifest::{GameContentManifest, PackMetadata},
    repository::LocalGameContentState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ManifestState {
    Fresh,
    CachedStale,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GameContentFindingKind {
    Game,
    Pack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GameContentHealthState {
    Current,
    UpdateAvailable,
    GameUpdateRequired,
    MetadataStale,
    LocalIntegrityUncertain,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameContentEvidence {
    pub(crate) source: String,
    pub(crate) source_url: Option<String>,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameContentHealthFinding {
    pub(crate) kind: GameContentFindingKind,
    pub(crate) target_id: String,
    pub(crate) state: GameContentHealthState,
    pub(crate) disputed: bool,
    pub(crate) manifest_stale: bool,
    pub(crate) current_version: Option<String>,
    pub(crate) required_version: Option<String>,
    pub(crate) reason: String,
    pub(crate) evidence: Vec<GameContentEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameContentHealthSnapshot {
    pub(crate) manifest_state: ManifestState,
    pub(crate) manifest_version: Option<String>,
    pub(crate) source_identity: Option<String>,
    pub(crate) source_url: Option<String>,
    pub(crate) detail: String,
    pub(crate) game: Option<GameContentHealthFinding>,
    pub(crate) packs: Vec<GameContentHealthFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedGameVersion {
    version: Option<String>,
    disputed: bool,
    evidence: Vec<GameContentEvidence>,
}

pub(crate) fn evaluate(
    local: &LocalGameContentState,
    manifest: &GameContentManifest,
    manifest_state: ManifestState,
    detail: String,
) -> GameContentHealthSnapshot {
    let manifest_stale = manifest_state == ManifestState::CachedStale;
    let resolved = resolve_game_version(local, manifest);

    let game = Some(game_finding(
        &resolved,
        manifest,
        manifest_stale,
    ));
    let packs = local
        .packs
        .iter()
        .map(|pack| {
            pack_finding(
                &pack.code,
                &pack.local_state,
                &resolved,
                manifest,
                manifest_stale,
            )
        })
        .collect();

    GameContentHealthSnapshot {
        manifest_state,
        manifest_version: Some(manifest.provenance.manifest_version.clone()),
        source_identity: Some(manifest.provenance.source_identity.clone()),
        source_url: manifest.provenance.source_url.clone(),
        detail,
        game,
        packs,
    }
}

pub(crate) fn missing_manifest(
    local: &LocalGameContentState,
    detail: String,
) -> GameContentHealthSnapshot {
    let game = Some(GameContentHealthFinding {
        kind: GameContentFindingKind::Game,
        target_id: "game".to_string(),
        state: GameContentHealthState::Unknown,
        disputed: false,
        manifest_stale: false,
        current_version: local.game_version.clone(),
        required_version: None,
        reason: "No trusted Game/DLC manifest is available; update state cannot be concluded."
            .to_string(),
        evidence: Vec::new(),
    });

    let packs = local
        .packs
        .iter()
        .map(|pack| GameContentHealthFinding {
            kind: GameContentFindingKind::Pack,
            target_id: pack.code.clone(),
            state: if pack.local_state == "installed" {
                GameContentHealthState::Unknown
            } else {
                GameContentHealthState::LocalIntegrityUncertain
            },
            disputed: false,
            manifest_stale: false,
            current_version: local.game_version.clone(),
            required_version: None,
            reason: if pack.local_state == "installed" {
                "Pack is present locally, but compatibility metadata is unavailable.".to_string()
            } else {
                "Pack files are incomplete or could not be inspected reliably.".to_string()
            },
            evidence: Vec::new(),
        })
        .collect();

    GameContentHealthSnapshot {
        manifest_state: ManifestState::Missing,
        manifest_version: None,
        source_identity: None,
        source_url: None,
        detail,
        game,
        packs,
    }
}

fn resolve_game_version(
    local: &LocalGameContentState,
    manifest: &GameContentManifest,
) -> ResolvedGameVersion {
    let fingerprint_versions = matching_fingerprint_versions(local, manifest);

    if let Some(explicit) = local.game_version.as_ref() {
        if fingerprint_versions
            .iter()
            .any(|version| version != explicit)
        {
            return ResolvedGameVersion {
                version: None,
                disputed: true,
                evidence: vec![GameContentEvidence {
                    source: manifest.provenance.source_identity.clone(),
                    source_url: manifest.provenance.source_url.clone(),
                    detail: format!(
                        "Local version {explicit} conflicts with sentinel evidence for {}.",
                        fingerprint_versions.iter().cloned().collect::<Vec<_>>().join(", ")
                    ),
                }],
            };
        }

        return ResolvedGameVersion {
            version: Some(explicit.clone()),
            disputed: false,
            evidence: vec![GameContentEvidence {
                source: "local-default-ini".to_string(),
                source_url: None,
                detail: format!("Installed game version {explicit} was read locally."),
            }],
        };
    }

    if fingerprint_versions.len() == 1 {
        let version = fingerprint_versions
            .iter()
            .next()
            .expect("one fingerprint version")
            .clone();
        return ResolvedGameVersion {
            version: Some(version.clone()),
            disputed: false,
            evidence: vec![GameContentEvidence {
                source: manifest.provenance.source_identity.clone(),
                source_url: manifest.provenance.source_url.clone(),
                detail: format!("Sentinel fingerprints resolve uniquely to game build {version}."),
            }],
        };
    }

    if fingerprint_versions.len() > 1 {
        return ResolvedGameVersion {
            version: None,
            disputed: true,
            evidence: vec![GameContentEvidence {
                source: manifest.provenance.source_identity.clone(),
                source_url: manifest.provenance.source_url.clone(),
                detail: format!(
                    "Sentinel fingerprints match multiple game builds: {}.",
                    fingerprint_versions.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
            }],
        };
    }

    ResolvedGameVersion {
        version: None,
        disputed: false,
        evidence: Vec::new(),
    }
}

fn matching_fingerprint_versions(
    local: &LocalGameContentState,
    manifest: &GameContentManifest,
) -> BTreeSet<String> {
    let local_by_path = local
        .sentinels
        .iter()
        .map(|item| {
            (
                item.relative_path.replace('\\', "/").to_ascii_lowercase(),
                item.sha256.to_ascii_lowercase(),
            )
        })
        .collect::<HashMap<_, _>>();

    manifest
        .game_builds
        .iter()
        .filter(|build| {
            build.fingerprints.iter().any(|fingerprint| {
                local_by_path
                    .get(&fingerprint.relative_path.to_ascii_lowercase())
                    .is_some_and(|sha256| sha256 == &fingerprint.sha256)
            })
        })
        .map(|build| build.version.clone())
        .collect()
}

fn game_finding(
    resolved: &ResolvedGameVersion,
    manifest: &GameContentManifest,
    manifest_stale: bool,
) -> GameContentHealthFinding {
    if resolved.disputed {
        return GameContentHealthFinding {
            kind: GameContentFindingKind::Game,
            target_id: "game".to_string(),
            state: GameContentHealthState::Unknown,
            disputed: true,
            manifest_stale,
            current_version: None,
            required_version: Some(manifest.latest_game_build.clone()),
            reason: "Game-build evidence conflicts, so no update conclusion is safe.".to_string(),
            evidence: resolved.evidence.clone(),
        };
    }

    let Some(current) = resolved.version.as_ref() else {
        return GameContentHealthFinding {
            kind: GameContentFindingKind::Game,
            target_id: "game".to_string(),
            state: GameContentHealthState::Unknown,
            disputed: false,
            manifest_stale,
            current_version: None,
            required_version: Some(manifest.latest_game_build.clone()),
            reason: "Installed game build could not be resolved from trusted local evidence."
                .to_string(),
            evidence: resolved.evidence.clone(),
        };
    };

    let comparison = compare_versions(current, &manifest.latest_game_build);
    let (state, reason) = match comparison {
        Some(Ordering::Less) => (
            GameContentHealthState::UpdateAvailable,
            if manifest_stale {
                format!(
                    "Cached metadata indicates game build {} is newer than installed {}.",
                    manifest.latest_game_build, current
                )
            } else {
                format!(
                    "Game build {} is newer than installed {}.",
                    manifest.latest_game_build, current
                )
            },
        ),
        Some(Ordering::Equal) if manifest_stale => (
            GameContentHealthState::MetadataStale,
            "Installed game matches the cached latest build, but Registry metadata is stale."
                .to_string(),
        ),
        Some(Ordering::Equal) => (
            GameContentHealthState::Current,
            "Installed game matches the latest known build.".to_string(),
        ),
        Some(Ordering::Greater) => (
            GameContentHealthState::MetadataStale,
            format!(
                "Installed build {current} is newer than manifest latest {}; metadata needs refresh.",
                manifest.latest_game_build
            ),
        ),
        None => (
            GameContentHealthState::Unknown,
            "Game versions could not be compared safely.".to_string(),
        ),
    };

    let mut evidence = resolved.evidence.clone();
    evidence.push(GameContentEvidence {
        source: manifest.provenance.source_identity.clone(),
        source_url: manifest.provenance.source_url.clone(),
        detail: format!(
            "Manifest latest game build is {}.",
            manifest.latest_game_build
        ),
    });

    GameContentHealthFinding {
        kind: GameContentFindingKind::Game,
        target_id: "game".to_string(),
        state,
        disputed: false,
        manifest_stale,
        current_version: Some(current.clone()),
        required_version: Some(manifest.latest_game_build.clone()),
        reason,
        evidence,
    }
}

fn pack_finding(
    code: &str,
    local_state: &str,
    resolved: &ResolvedGameVersion,
    manifest: &GameContentManifest,
    manifest_stale: bool,
) -> GameContentHealthFinding {
    if local_state != "installed" {
        return GameContentHealthFinding {
            kind: GameContentFindingKind::Pack,
            target_id: code.to_string(),
            state: GameContentHealthState::LocalIntegrityUncertain,
            disputed: false,
            manifest_stale,
            current_version: resolved.version.clone(),
            required_version: None,
            reason: "Pack files are incomplete or could not be inspected reliably.".to_string(),
            evidence: Vec::new(),
        };
    }

    let entries = manifest
        .packs
        .iter()
        .filter(|entry| entry.code.eq_ignore_ascii_case(code))
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return GameContentHealthFinding {
            kind: GameContentFindingKind::Pack,
            target_id: code.to_string(),
            state: GameContentHealthState::Unknown,
            disputed: false,
            manifest_stale,
            current_version: resolved.version.clone(),
            required_version: None,
            reason: "Pack is installed, but the manifest has no compatibility metadata for it."
                .to_string(),
            evidence: Vec::new(),
        };
    }

    if metadata_is_disputed(&entries) {
        return GameContentHealthFinding {
            kind: GameContentFindingKind::Pack,
            target_id: code.to_string(),
            state: GameContentHealthState::Unknown,
            disputed: true,
            manifest_stale,
            current_version: resolved.version.clone(),
            required_version: None,
            reason: "Trusted metadata sources disagree about this pack's compatibility requirements."
                .to_string(),
            evidence: pack_evidence(&entries),
        };
    }

    let entry = entries[0];
    let required = entry.min_game_version.clone();

    if resolved.disputed {
        return GameContentHealthFinding {
            kind: GameContentFindingKind::Pack,
            target_id: code.to_string(),
            state: GameContentHealthState::Unknown,
            disputed: true,
            manifest_stale,
            current_version: None,
            required_version: required,
            reason: "Game-build evidence is disputed, so pack compatibility cannot be concluded."
                .to_string(),
            evidence: pack_evidence(&entries),
        };
    }

    if let (Some(current), Some(minimum)) =
        (resolved.version.as_ref(), entry.min_game_version.as_ref())
    {
        match compare_versions(current, minimum) {
            Some(Ordering::Less) => {
                return GameContentHealthFinding {
                    kind: GameContentFindingKind::Pack,
                    target_id: code.to_string(),
                    state: GameContentHealthState::GameUpdateRequired,
                    disputed: false,
                    manifest_stale,
                    current_version: Some(current.clone()),
                    required_version: Some(minimum.clone()),
                    reason: format!(
                        "{code} requires game build {minimum} or newer; installed build is {current}."
                    ),
                    evidence: pack_evidence(&entries),
                };
            }
            None => {
                return GameContentHealthFinding {
                    kind: GameContentFindingKind::Pack,
                    target_id: code.to_string(),
                    state: GameContentHealthState::Unknown,
                    disputed: false,
                    manifest_stale,
                    current_version: Some(current.clone()),
                    required_version: Some(minimum.clone()),
                    reason: "Pack minimum version could not be compared safely.".to_string(),
                    evidence: pack_evidence(&entries),
                };
            }
            _ => {}
        }
    } else if entry.min_game_version.is_some() && resolved.version.is_none() {
        return GameContentHealthFinding {
            kind: GameContentFindingKind::Pack,
            target_id: code.to_string(),
            state: GameContentHealthState::Unknown,
            disputed: false,
            manifest_stale,
            current_version: None,
            required_version: required,
            reason: "Pack has a minimum game build, but the installed game build is unknown."
                .to_string(),
            evidence: pack_evidence(&entries),
        };
    }

    GameContentHealthFinding {
        kind: GameContentFindingKind::Pack,
        target_id: code.to_string(),
        state: if manifest_stale {
            GameContentHealthState::MetadataStale
        } else {
            GameContentHealthState::Current
        },
        disputed: false,
        manifest_stale,
        current_version: resolved.version.clone(),
        required_version: required,
        reason: if manifest_stale {
            "Cached metadata does not show a compatibility problem, but it is stale.".to_string()
        } else {
            "Installed game satisfies the pack's known compatibility requirement.".to_string()
        },
        evidence: pack_evidence(&entries),
    }
}

fn metadata_is_disputed(entries: &[&PackMetadata]) -> bool {
    let signatures = entries
        .iter()
        .map(|entry| {
            (
                entry.pack_kind.as_str(),
                entry.min_game_version.as_deref(),
            )
        })
        .collect::<BTreeSet<_>>();

    signatures.len() > 1
}

fn pack_evidence(entries: &[&PackMetadata]) -> Vec<GameContentEvidence> {
    entries
        .iter()
        .map(|entry| GameContentEvidence {
            source: entry.evidence_source.clone(),
            source_url: entry.evidence_url.clone(),
            detail: match entry.min_game_version.as_ref() {
                Some(version) => format!("Minimum game build: {version}."),
                None => "No minimum game build is declared.".to_string(),
            },
        })
        .collect()
}

fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    Some(parse_version(left)?.cmp(&parse_version(right)?))
}

fn parse_version(value: &str) -> Option<[u32; 4]> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 4 {
        return None;
    }

    let mut parsed = [0_u32; 4];
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.chars().all(|character| character.is_ascii_digit()) {
            return None;
        }
        parsed[index] = part.parse().ok()?;
    }
    Some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::content_health::manifest::{
        GameBuildMetadata, ManifestFingerprint, ManifestProvenance, PackMetadata,
    };
    use crate::game::content_inventory::SentinelFingerprint;
    use crate::game::content_health::repository::LocalPackState;

    fn manifest() -> GameContentManifest {
        GameContentManifest {
            latest_game_build: "1.128.90.1030".to_string(),
            game_builds: vec![
                GameBuildMetadata {
                    version: "1.127.0.1000".to_string(),
                    released_at: None,
                    fingerprints: vec![ManifestFingerprint {
                        relative_path: "Game/Bin/TS4_x64.exe".to_string(),
                        sha256: "1".repeat(64),
                    }],
                },
                GameBuildMetadata {
                    version: "1.128.90.1030".to_string(),
                    released_at: None,
                    fingerprints: vec![ManifestFingerprint {
                        relative_path: "Game/Bin/TS4_x64.exe".to_string(),
                        sha256: "2".repeat(64),
                    }],
                },
            ],
            packs: vec![PackMetadata {
                code: "EP99".to_string(),
                pack_kind: "expansion".to_string(),
                min_game_version: Some("1.128.0.0".to_string()),
                released_at: None,
                expected_fingerprints: Vec::new(),
                evidence_source: "official".to_string(),
                evidence_url: None,
            }],
            provenance: ManifestProvenance {
                manifest_version: "m1".to_string(),
                source_identity: "registry".to_string(),
                source_url: None,
                retrieved_at: "2026-09-28T08:00:00Z".to_string(),
                expires_at: None,
                checksum_sha256: None,
                signature: None,
            },
        }
    }

    #[test]
    fn older_game_build_is_update_available() {
        let local = LocalGameContentState {
            game_version: Some("1.127.0.1000".to_string()),
            sentinels: Vec::new(),
            packs: Vec::new(),
        };

        let result = evaluate(&local, &manifest(), ManifestState::Fresh, "live".to_string());
        assert_eq!(
            result.game.expect("game").state,
            GameContentHealthState::UpdateAvailable
        );
    }

    #[test]
    fn pack_minimum_version_requires_game_update() {
        let local = LocalGameContentState {
            game_version: Some("1.127.0.1000".to_string()),
            sentinels: Vec::new(),
            packs: vec![LocalPackState {
                code: "EP99".to_string(),
                local_state: "installed".to_string(),
            }],
        };

        let result = evaluate(&local, &manifest(), ManifestState::Fresh, "live".to_string());
        assert_eq!(
            result.packs[0].state,
            GameContentHealthState::GameUpdateRequired
        );
    }

    #[test]
    fn sentinel_can_resolve_missing_local_version() {
        let local = LocalGameContentState {
            game_version: None,
            sentinels: vec![SentinelFingerprint {
                relative_path: "Game/Bin/TS4_x64.exe".to_string(),
                sha256: "2".repeat(64),
                size_bytes: 42,
            }],
            packs: Vec::new(),
        };

        let result = evaluate(&local, &manifest(), ManifestState::Fresh, "live".to_string());
        let game = result.game.expect("game");
        assert_eq!(game.current_version.as_deref(), Some("1.128.90.1030"));
        assert_eq!(game.state, GameContentHealthState::Current);
    }

    #[test]
    fn conflicting_pack_metadata_is_disputed_unknown() {
        let mut value = manifest();
        value.packs.push(PackMetadata {
            code: "EP99".to_string(),
            pack_kind: "expansion".to_string(),
            min_game_version: Some("1.129.0.0".to_string()),
            released_at: None,
            expected_fingerprints: Vec::new(),
            evidence_source: "second-source".to_string(),
            evidence_url: None,
        });

        let local = LocalGameContentState {
            game_version: Some("1.128.90.1030".to_string()),
            sentinels: Vec::new(),
            packs: vec![LocalPackState {
                code: "EP99".to_string(),
                local_state: "installed".to_string(),
            }],
        };

        let result = evaluate(&local, &value, ManifestState::Fresh, "live".to_string());
        assert_eq!(result.packs[0].state, GameContentHealthState::Unknown);
        assert!(result.packs[0].disputed);
    }

    #[test]
    fn cached_manifest_remains_usable_but_visible_as_stale() {
        let local = LocalGameContentState {
            game_version: Some("1.128.90.1030".to_string()),
            sentinels: Vec::new(),
            packs: vec![LocalPackState {
                code: "EP99".to_string(),
                local_state: "installed".to_string(),
            }],
        };

        let result = evaluate(
            &local,
            &manifest(),
            ManifestState::CachedStale,
            "offline cache".to_string(),
        );

        assert_eq!(result.manifest_state, ManifestState::CachedStale);
        assert_eq!(
            result.packs[0].state,
            GameContentHealthState::MetadataStale
        );
    }
}
