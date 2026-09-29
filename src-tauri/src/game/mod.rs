mod content_health;
mod content_inventory;
mod provider_update;

pub(crate) use content_health::{evaluate_game_content_health, GameContentHealthSnapshot};
pub(crate) use content_inventory::{
    discover_game_content, inspect_game_content_path, persist_game_content_path,
    refresh_and_persist_game_content,
    GameContentInstallation, GameContentRepository, GameContentSnapshot,
    SqliteGameContentRepository,
};
pub(crate) use provider_update::{
    begin_verification as begin_provider_update_verification,
    complete_verification as complete_provider_update_verification,
    current_program_version_for_session, fail_verification as fail_provider_update_verification,
    get_capability as get_provider_update_capability,
    get_pending_session as get_pending_provider_update_session,
    get_session as get_provider_update_session, latest_mod_user_root, start_provider_update,
    sync_latest_mod_game_version, ProviderUpdateCapability, ProviderUpdateSessionView,
    ProviderUpdateState, ProviderUpdateTargetKind,
};

use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

const SIMS_RELATIVE_PATH: [&str; 2] = ["Electronic Arts", "The Sims 4"];
const GAME_VERSION_FILE: &str = "GameVersion.txt";
const MODS_DIRECTORY: &str = "Mods";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameVersion {
    pub(crate) major: u32,
    pub(crate) minor: u32,
    pub(crate) patch: u32,
    pub(crate) build: u32,
    pub(crate) normalized: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum VersionState {
    Available { version: GameVersion },
    Missing,
    Invalid { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DiscoverySource {
    KnownDocuments,
    OneDriveFallback,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstallationCandidate {
    pub(crate) root: PathBuf,
    pub(crate) mods_root: PathBuf,
    pub(crate) source: DiscoverySource,
    pub(crate) mods_available: bool,
    pub(crate) version: VersionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum ManualInspection {
    Available { installation: InstallationCandidate },
    Unavailable { reason: String },
}

pub(crate) fn discover_installations() -> Vec<InstallationCandidate> {
    discover_from_document_roots(candidate_document_roots())
}

pub(crate) fn inspect_manual_path(path: &Path) -> ManualInspection {
    if !path.is_dir() {
        return ManualInspection::Unavailable {
            reason: "Selected path is not an existing directory.".to_string(),
        };
    }

    let root = normalize_selected_root(path);
    if !root.is_dir() {
        return ManualInspection::Unavailable {
            reason: "Selected directory does not contain a The Sims 4 user folder.".to_string(),
        };
    }

    ManualInspection::Available {
        installation: inspect_root(&root, DiscoverySource::Manual),
    }
}

fn candidate_document_roots() -> Vec<(PathBuf, DiscoverySource)> {
    let mut roots = Vec::new();

    if let Some(documents) = dirs::document_dir() {
        roots.push((documents, DiscoverySource::KnownDocuments));
    }

    for variable in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        if let Some(one_drive_root) = env::var_os(variable) {
            roots.push((
                PathBuf::from(one_drive_root).join("Documents"),
                DiscoverySource::OneDriveFallback,
            ));
        }
    }

    deduplicate_roots(roots)
}

fn deduplicate_roots(roots: Vec<(PathBuf, DiscoverySource)>) -> Vec<(PathBuf, DiscoverySource)> {
    let mut seen = HashSet::new();

    roots
        .into_iter()
        .filter(|(path, _)| seen.insert(path.clone()))
        .collect()
}

fn discover_from_document_roots(
    document_roots: Vec<(PathBuf, DiscoverySource)>,
) -> Vec<InstallationCandidate> {
    let mut seen = HashSet::new();
    let mut installations = Vec::new();

    for (documents, source) in document_roots {
        let root = sims_root_from_documents(&documents);
        if root.is_dir() && seen.insert(root.clone()) {
            installations.push(inspect_root(&root, source));
        }
    }

    installations
}

fn sims_root_from_documents(documents: &Path) -> PathBuf {
    SIMS_RELATIVE_PATH
        .iter()
        .fold(documents.to_path_buf(), |current, segment| {
            current.join(segment)
        })
}

fn normalize_selected_root(path: &Path) -> PathBuf {
    if is_sims_user_root(path) {
        return path.to_path_buf();
    }

    if path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(MODS_DIRECTORY))
    {
        if let Some(parent) = path.parent() {
            if is_sims_user_root(parent) {
                return parent.to_path_buf();
            }
        }
    }

    let nested = sims_root_from_documents(path);
    if nested.is_dir() {
        return nested;
    }

    path.to_path_buf()
}

fn is_sims_user_root(path: &Path) -> bool {
    path.join(MODS_DIRECTORY).is_dir() || path.join(GAME_VERSION_FILE).is_file()
}

fn inspect_root(root: &Path, source: DiscoverySource) -> InstallationCandidate {
    let mods_root = root.join(MODS_DIRECTORY);

    InstallationCandidate {
        root: root.to_path_buf(),
        mods_available: mods_root.is_dir(),
        mods_root,
        source,
        version: read_game_version(&root.join(GAME_VERSION_FILE)),
    }
}

fn read_game_version(path: &Path) -> VersionState {
    match fs::read_to_string(path) {
        Ok(content) => match parse_game_version(&content) {
            Ok(version) => VersionState::Available { version },
            Err(reason) => VersionState::Invalid { reason },
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => VersionState::Missing,
        Err(error) => VersionState::Invalid {
            reason: format!("Unable to read GameVersion.txt: {error}"),
        },
    }
}

fn parse_game_version(content: &str) -> Result<GameVersion, String> {
    let raw = content
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| "GameVersion.txt is empty.".to_string())?;

    let value = raw
        .split_once('=')
        .map(|(_, right)| right.trim())
        .unwrap_or(raw)
        .trim();

    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err("Game version must contain exactly four numeric components.".to_string());
    }

    let mut numbers = [0_u32; 4];
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.chars().all(|character| character.is_ascii_digit()) {
            return Err("Game version contains a non-numeric component.".to_string());
        }

        numbers[index] = part
            .parse::<u32>()
            .map_err(|_| "Game version component is outside the supported range.".to_string())?;
    }

    Ok(GameVersion {
        major: numbers[0],
        minor: numbers[1],
        patch: numbers[2],
        build: numbers[3],
        normalized: format!(
            "{}.{}.{}.{}",
            numbers[0], numbers[1], numbers[2], numbers[3]
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_sims_root() -> TempDir {
        let temp = TempDir::new().expect("create temp directory");
        fs::create_dir_all(
            temp.path()
                .join("Electronic Arts")
                .join("The Sims 4")
                .join("Mods"),
        )
        .expect("create Sims Mods directory");
        temp
    }

    #[test]
    fn parses_labeled_game_version() {
        let version =
            parse_game_version("GameVersion = 1.128.90.1030\n").expect("valid labeled version");

        assert_eq!(version.normalized, "1.128.90.1030");
        assert_eq!(
            (version.major, version.minor, version.patch, version.build),
            (1, 128, 90, 1030)
        );
    }

    #[test]
    fn parses_raw_game_version_and_normalizes_leading_zeroes() {
        let version = parse_game_version("\u{feff}01.128.090.1030\r\n").expect("valid raw version");

        assert_eq!(version.normalized, "1.128.90.1030");
    }

    #[test]
    fn malformed_version_is_rejected_without_guessing() {
        let error = parse_game_version("GameVersion = 1.128.broken.1030")
            .expect_err("malformed version must fail");

        assert!(error.contains("non-numeric"));
    }

    #[test]
    fn missing_version_file_is_reported_as_missing() {
        let temp = create_sims_root();
        let root = temp.path().join("Electronic Arts").join("The Sims 4");

        let candidate = inspect_root(&root, DiscoverySource::KnownDocuments);

        assert!(candidate.mods_available);
        assert_eq!(candidate.version, VersionState::Missing);
    }

    #[test]
    fn discovers_common_documents_layout() {
        let temp = create_sims_root();
        let root = temp.path().join("Electronic Arts").join("The Sims 4");
        fs::write(root.join(GAME_VERSION_FILE), "GameVersion = 1.128.90.1030")
            .expect("write version fixture");

        let discovered = discover_from_document_roots(vec![(
            temp.path().to_path_buf(),
            DiscoverySource::KnownDocuments,
        )]);

        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].root, root);
        assert!(matches!(
            discovered[0].version,
            VersionState::Available { .. }
        ));
    }

    #[test]
    fn redirected_documents_roots_are_deduplicated() {
        let temp = create_sims_root();
        let root = temp.path().to_path_buf();

        let discovered = discover_from_document_roots(vec![
            (root.clone(), DiscoverySource::KnownDocuments),
            (root, DiscoverySource::OneDriveFallback),
        ]);

        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].source, DiscoverySource::KnownDocuments);
    }

    #[test]
    fn manual_override_accepts_direct_sims_root() {
        let temp = create_sims_root();
        let root = temp.path().join("Electronic Arts").join("The Sims 4");
        fs::write(root.join(GAME_VERSION_FILE), "1.128.90.1030").expect("write version fixture");

        let result = inspect_manual_path(&root);

        assert!(matches!(
            result,
            ManualInspection::Available {
                installation: InstallationCandidate {
                    source: DiscoverySource::Manual,
                    ..
                }
            }
        ));
    }

    #[test]
    fn manual_override_accepts_documents_parent() {
        let temp = create_sims_root();

        let result = inspect_manual_path(temp.path());

        match result {
            ManualInspection::Available { installation } => {
                assert_eq!(
                    installation.root,
                    temp.path().join("Electronic Arts").join("The Sims 4")
                );
            }
            ManualInspection::Unavailable { reason } => {
                panic!("expected available installation, got {reason}");
            }
        }
    }

    #[test]
    fn manual_override_accepts_direct_mods_folder() {
        let temp = create_sims_root();
        let sims_root = temp.path().join("Electronic Arts").join("The Sims 4");
        let mods_root = sims_root.join("Mods");

        let result = inspect_manual_path(&mods_root);

        match result {
            ManualInspection::Available { installation } => {
                assert_eq!(installation.root, sims_root);
                assert_eq!(installation.mods_root, mods_root);
                assert!(installation.mods_available);
            }
            ManualInspection::Unavailable { reason } => {
                panic!("expected available installation, got {reason}");
            }
        }
    }

    #[test]
    fn manual_override_reports_nonexistent_path() {
        let temp = TempDir::new().expect("create temp directory");
        let result = inspect_manual_path(&temp.path().join("does-not-exist"));

        assert!(matches!(result, ManualInspection::Unavailable { .. }));
    }
}
