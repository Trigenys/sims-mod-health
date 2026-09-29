mod packs;
mod providers;
mod repository;
mod version;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::storage;

use super::GameVersion;
use packs::LocalPackProbe;
use providers::{infer_provider_from_path, is_game_install_root, EaAppProbe, SteamProbe};
pub(crate) use repository::{
    latest_game_content_version, GameContentRepository, SqliteGameContentRepository,
};
use version::LocalVersionProbe;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GameProvider {
    EaApp,
    Steam,
    Unknown,
}

impl GameProvider {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::EaApp => "ea_app",
            Self::Steam => "steam",
            Self::Unknown => "unknown",
        }
    }

    fn priority(self) -> u8 {
        match self {
            Self::Steam => 3,
            Self::EaApp => 2,
            Self::Unknown => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderEvidence {
    Registry,
    SteamManifest,
    DefaultPath,
    Manual,
}

impl ProviderEvidence {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Registry => "registry",
            Self::SteamManifest => "steam_manifest",
            Self::DefaultPath => "default_path",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceConfidence {
    Definitive,
    Probable,
    Unknown,
}

impl EvidenceConfidence {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Definitive => "definitive",
            Self::Probable => "probable",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VersionEvidenceKind {
    DefaultIni,
    SentinelFingerprint,
    Unknown,
}

impl VersionEvidenceKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::DefaultIni => "default_ini",
            Self::SentinelFingerprint => "sentinel_fingerprint",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SentinelFingerprint {
    pub(crate) relative_path: String,
    pub(crate) sha256: String,
    pub(crate) size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameBuildEvidence {
    pub(crate) version: Option<GameVersion>,
    pub(crate) evidence_kind: VersionEvidenceKind,
    pub(crate) confidence: EvidenceConfidence,
    pub(crate) sentinels: Vec<SentinelFingerprint>,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PackKind {
    Expansion,
    Game,
    StuffOrKit,
    Free,
    Kit,
    Unknown,
}

impl PackKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Expansion => "expansion",
            Self::Game => "game",
            Self::StuffOrKit => "stuff_or_kit",
            Self::Free => "free",
            Self::Kit => "kit",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PackLocalState {
    Installed,
    Partial,
    Unknown,
}

impl PackLocalState {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Partial => "partial",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstalledPackObservation {
    pub(crate) pack_code: String,
    pub(crate) pack_kind: PackKind,
    pub(crate) local_state: PackLocalState,
    pub(crate) size_bytes: Option<i64>,
    pub(crate) marker_count: i64,
    pub(crate) observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameContentInstallation {
    pub(crate) install_root: PathBuf,
    pub(crate) provider: GameProvider,
    pub(crate) provider_evidence: ProviderEvidence,
    pub(crate) build: GameBuildEvidence,
    pub(crate) packs: Vec<InstalledPackObservation>,
    pub(crate) observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameContentSnapshot {
    pub(crate) installations: Vec<GameContentInstallation>,
}

#[derive(Debug, Clone)]
pub(super) struct ProviderCandidate {
    pub(super) root: PathBuf,
    pub(super) provider: GameProvider,
    pub(super) evidence: ProviderEvidence,
}

pub(super) trait InstallationProviderProbe {
    fn discover(&self) -> Vec<ProviderCandidate>;
}

pub(crate) trait GameVersionProbe {
    fn probe(&self, install_root: &Path) -> GameBuildEvidence;
}

pub(crate) trait PackInventoryProbe {
    fn probe(&self, install_root: &Path, observed_at: &str) -> Vec<InstalledPackObservation>;
}

pub(crate) fn refresh_and_persist_game_content(
    database_path: &Path,
) -> Result<GameContentSnapshot, String> {
    let snapshot = discover_game_content();
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;
    SqliteGameContentRepository::new(&connection)
        .persist_snapshot(&snapshot)
        .map_err(|error| error.to_string())?;
    Ok(snapshot)
}

pub(crate) fn discover_game_content() -> GameContentSnapshot {
    let mut candidates = Vec::new();
    candidates.extend(SteamProbe.discover());
    candidates.extend(EaAppProbe.discover());

    let mut by_root: BTreeMap<String, ProviderCandidate> = BTreeMap::new();
    for candidate in candidates {
        if !is_game_install_root(&candidate.root) {
            continue;
        }

        let key = candidate.root.to_string_lossy().to_ascii_lowercase();
        match by_root.get(&key) {
            Some(existing) if existing.provider.priority() >= candidate.provider.priority() => {}
            _ => {
                by_root.insert(key, candidate);
            }
        }
    }

    let version_probe = LocalVersionProbe;
    let pack_probe = LocalPackProbe;
    let observed_at = observation_timestamp();

    GameContentSnapshot {
        installations: by_root
            .into_values()
            .map(|candidate| {
                inspect_candidate(&candidate, &version_probe, &pack_probe, &observed_at)
            })
            .collect(),
    }
}

pub(crate) fn persist_game_content_path(
    database_path: &Path,
    path: &Path,
) -> Result<GameContentInstallation, String> {
    let installation = inspect_game_content_path(path)?;
    let snapshot = GameContentSnapshot {
        installations: vec![installation.clone()],
    };
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;
    SqliteGameContentRepository::new(&connection)
        .persist_snapshot(&snapshot)
        .map_err(|error| error.to_string())?;
    Ok(installation)
}

pub(crate) fn inspect_game_content_path(path: &Path) -> Result<GameContentInstallation, String> {
    if !is_game_install_root(path) {
        return Err(
            "Selected directory is not a recognizable The Sims 4 game installation.".to_string(),
        );
    }

    let candidate = ProviderCandidate {
        root: path.to_path_buf(),
        provider: infer_provider_from_path(path),
        evidence: ProviderEvidence::Manual,
    };
    let observed_at = observation_timestamp();

    Ok(inspect_candidate(
        &candidate,
        &LocalVersionProbe,
        &LocalPackProbe,
        &observed_at,
    ))
}

fn inspect_candidate(
    candidate: &ProviderCandidate,
    version_probe: &impl GameVersionProbe,
    pack_probe: &impl PackInventoryProbe,
    observed_at: &str,
) -> GameContentInstallation {
    GameContentInstallation {
        install_root: candidate.root.clone(),
        provider: candidate.provider,
        provider_evidence: candidate.evidence,
        build: version_probe.probe(&candidate.root),
        packs: pack_probe.probe(&candidate.root, observed_at),
        observed_at: observed_at.to_string(),
    }
}

fn observation_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!("unix:{seconds}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_game_root() -> TempDir {
        let temp = TempDir::new().expect("create temp root");
        fs::create_dir_all(temp.path().join("Game").join("Bin")).expect("create bin");
        fs::create_dir_all(temp.path().join("Data").join("Client")).expect("create client");
        fs::write(
            temp.path().join("Game").join("Bin").join("TS4_x64.exe"),
            b"fixture",
        )
        .expect("write exe fixture");
        temp
    }

    #[test]
    fn manual_game_selection_persists_custom_installation() {
        let game = create_game_root();
        fs::write(
            game.path().join("Game").join("Bin").join("Default.ini"),
            "[Version]\ngameversion = 1.128.90.1030\n",
        )
        .expect("default ini");

        let database = TempDir::new().expect("database temp");
        let database_path = database.path().join("setup.sqlite3");
        crate::storage::initialize(&database_path).expect("initialize database");

        let selected =
            persist_game_content_path(&database_path, game.path()).expect("persist custom game");

        assert_eq!(selected.install_root, game.path());
        assert_eq!(
            selected
                .build
                .version
                .as_ref()
                .map(|version| version.normalized.as_str()),
            Some("1.128.90.1030")
        );

        let connection = crate::storage::open(&database_path).expect("open database");
        let stored: (String, String) = connection
            .query_row(
                "SELECT install_root, game_version
                 FROM game_content_installations
                 LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("stored installation");

        assert_eq!(stored.0, game.path().to_string_lossy());
        assert_eq!(stored.1, "1.128.90.1030");
    }

    #[test]
    fn custom_ea_and_steam_install_paths_keep_their_provider_identity() {
        let temp = TempDir::new().expect("custom provider temp");
        let cases = [
            (
                temp.path().join("Custom").join("EA Games").join("The Sims 4"),
                GameProvider::EaApp,
            ),
            (
                temp.path()
                    .join("SteamLibrary")
                    .join("steamapps")
                    .join("common")
                    .join("The Sims 4"),
                GameProvider::Steam,
            ),
        ];

        for (root, expected_provider) in cases {
            fs::create_dir_all(root.join("Game").join("Bin")).expect("custom game bin");
            fs::create_dir_all(root.join("Data").join("Client")).expect("custom client data");
            fs::write(
                root.join("Game").join("Bin").join("TS4_x64.exe"),
                b"synthetic-fixture",
            )
            .expect("custom game marker");
            fs::write(
                root.join("Game").join("Bin").join("Default.ini"),
                "[Version]\ngameversion = 1.128.90.1030\n",
            )
            .expect("custom version marker");

            let inspected = inspect_game_content_path(&root).expect("inspect custom path");

            assert_eq!(inspected.install_root, root);
            assert_eq!(inspected.provider, expected_provider);
            assert_eq!(
                inspected
                    .build
                    .version
                    .as_ref()
                    .map(|version| version.normalized.as_str()),
                Some("1.128.90.1030")
            );
        }
    }

    #[test]
    fn manual_inspection_uses_unknown_provider_for_neutral_path() {
        let game = create_game_root();
        fs::write(
            game.path().join("Game").join("Bin").join("Default.ini"),
            "[Version]\ngameversion = 1.128.90.1030\n",
        )
        .expect("default ini");

        let inspected = inspect_game_content_path(game.path()).expect("inspect");
        assert_eq!(inspected.provider, GameProvider::Unknown);
        assert_eq!(
            inspected
                .build
                .version
                .as_ref()
                .map(|version| version.normalized.as_str()),
            Some("1.128.90.1030")
        );
    }
}
