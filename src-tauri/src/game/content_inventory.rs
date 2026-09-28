use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::GameVersion;

const STEAM_APP_ID: &str = "1222670";
const MAX_PACK_WALK_ENTRIES: usize = 2_000_000;
const SENTINELS: &[&str] = &[
    "Game/Bin/Default.ini",
    "Game/Bin/TS4_x64.exe",
    "Delta/EP01/Version.ini",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GameProvider {
    EaApp,
    Steam,
    Unknown,
}

impl GameProvider {
    fn as_str(self) -> &'static str {
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
    fn as_str(self) -> &'static str {
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
    fn as_str(self) -> &'static str {
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
    fn as_str(self) -> &'static str {
        match self {
            Self::DefaultIni => "default_ini",
            Self::SentinelFingerprint => "sentinel_fingerprint",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    fn as_str(self) -> &'static str {
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
    fn as_str(self) -> &'static str {
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
    pub(crate) size_bytes: Option<u64>,
    pub(crate) marker_count: u64,
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
struct ProviderCandidate {
    root: PathBuf,
    provider: GameProvider,
    evidence: ProviderEvidence,
}

trait InstallationProviderProbe {
    fn discover(&self) -> Vec<ProviderCandidate>;
}

pub(crate) trait GameVersionProbe {
    fn probe(&self, install_root: &Path) -> GameBuildEvidence;
}

pub(crate) trait PackInventoryProbe {
    fn probe(&self, install_root: &Path, observed_at: &str) -> Vec<InstalledPackObservation>;
}

struct EaAppProbe;
struct SteamProbe;
struct LocalVersionProbe;
struct LocalPackProbe;

impl InstallationProviderProbe for EaAppProbe {
    fn discover(&self) -> Vec<ProviderCandidate> {
        let mut candidates = ea_registry_candidates();
        candidates.extend(default_ea_candidates());
        candidates
    }
}

impl InstallationProviderProbe for SteamProbe {
    fn discover(&self) -> Vec<ProviderCandidate> {
        discover_steam_candidates(steam_roots())
    }
}

impl GameVersionProbe for LocalVersionProbe {
    fn probe(&self, install_root: &Path) -> GameBuildEvidence {
        let default_ini = install_root.join("Game").join("Bin").join("Default.ini");
        let sentinels = fingerprint_sentinels(install_root);

        match fs::read_to_string(&default_ini) {
            Ok(content) => match parse_default_ini_game_version(&content) {
                Ok(version) => GameBuildEvidence {
                    version: Some(version),
                    evidence_kind: VersionEvidenceKind::DefaultIni,
                    confidence: EvidenceConfidence::Definitive,
                    sentinels,
                    detail: "Version parsed from Game/Bin/Default.ini.".to_string(),
                },
                Err(reason) if !sentinels.is_empty() => GameBuildEvidence {
                    version: None,
                    evidence_kind: VersionEvidenceKind::SentinelFingerprint,
                    confidence: EvidenceConfidence::Unknown,
                    sentinels,
                    detail: format!(
                        "Default.ini was present but unusable ({reason}); sentinel fingerprints were recorded without guessing a version."
                    ),
                },
                Err(reason) => GameBuildEvidence {
                    version: None,
                    evidence_kind: VersionEvidenceKind::Unknown,
                    confidence: EvidenceConfidence::Unknown,
                    sentinels,
                    detail: format!(
                        "Default.ini was present but unusable ({reason}) and no sentinel fingerprint was available."
                    ),
                },
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound && !sentinels.is_empty() => {
                GameBuildEvidence {
                    version: None,
                    evidence_kind: VersionEvidenceKind::SentinelFingerprint,
                    confidence: EvidenceConfidence::Unknown,
                    sentinels,
                    detail:
                        "No readable local version string; sentinel fingerprints were recorded for later manifest resolution."
                            .to_string(),
                }
            }
            Err(error) => GameBuildEvidence {
                version: None,
                evidence_kind: VersionEvidenceKind::Unknown,
                confidence: EvidenceConfidence::Unknown,
                sentinels,
                detail: format!("Unable to resolve the local game version: {error}"),
            },
        }
    }
}

impl PackInventoryProbe for LocalPackProbe {
    fn probe(&self, install_root: &Path, observed_at: &str) -> Vec<InstalledPackObservation> {
        inventory_packs(install_root, observed_at)
    }
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
            .map(|candidate| inspect_candidate(&candidate, &version_probe, &pack_probe, &observed_at))
            .collect(),
    }
}

pub(crate) fn inspect_game_content_path(path: &Path) -> Result<GameContentInstallation, String> {
    if !is_game_install_root(path) {
        return Err("Selected directory is not a recognizable The Sims 4 game installation.".to_string());
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

pub(crate) trait GameContentRepository {
    fn persist_snapshot(&self, snapshot: &GameContentSnapshot) -> rusqlite::Result<()>;
}

pub(crate) struct SqliteGameContentRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteGameContentRepository<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl GameContentRepository for SqliteGameContentRepository<'_> {
    fn persist_snapshot(&self, snapshot: &GameContentSnapshot) -> rusqlite::Result<()> {
        for installation in &snapshot.installations {
            let sentinel_json =
                serde_json::to_string(&installation.build.sentinels).unwrap_or_else(|_| "[]".to_string());

            self.connection.execute(
                "INSERT INTO game_content_installations (
                    install_root, provider, provider_evidence, game_version,
                    version_evidence_kind, version_confidence, sentinel_json,
                    first_seen_at, last_seen_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
                 ON CONFLICT(install_root) DO UPDATE SET
                    provider = excluded.provider,
                    provider_evidence = excluded.provider_evidence,
                    game_version = excluded.game_version,
                    version_evidence_kind = excluded.version_evidence_kind,
                    version_confidence = excluded.version_confidence,
                    sentinel_json = excluded.sentinel_json,
                    last_seen_at = excluded.last_seen_at",
                params![
                    installation.install_root.to_string_lossy(),
                    installation.provider.as_str(),
                    installation.provider_evidence.as_str(),
                    installation
                        .build
                        .version
                        .as_ref()
                        .map(|version| version.normalized.as_str()),
                    installation.build.evidence_kind.as_str(),
                    installation.build.confidence.as_str(),
                    sentinel_json,
                    installation.observed_at,
                ],
            )?;

            let installation_id: i64 = self.connection.query_row(
                "SELECT id FROM game_content_installations WHERE install_root = ?1",
                [installation.install_root.to_string_lossy().to_string()],
                |row| row.get(0),
            )?;

            self.connection.execute(
                "DELETE FROM installed_packs WHERE game_content_installation_id = ?1",
                [installation_id],
            )?;

            for pack in &installation.packs {
                self.connection.execute(
                    "INSERT INTO installed_packs (
                        game_content_installation_id, pack_code, pack_kind, local_state,
                        size_bytes, marker_count, observed_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        installation_id,
                        pack.pack_code,
                        pack.pack_kind.as_str(),
                        pack.local_state.as_str(),
                        pack.size_bytes,
                        pack.marker_count,
                        pack.observed_at,
                    ],
                )?;
            }
        }

        Ok(())
    }
}

fn parse_default_ini_game_version(content: &str) -> Result<GameVersion, String> {
    let value = content
        .lines()
        .map(str::trim)
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            if key.trim().eq_ignore_ascii_case("gameversion") {
                Some(value.trim())
            } else {
                None
            }
        })
        .ok_or_else(|| "gameversion key is missing".to_string())?;

    parse_version_value(value)
}

fn parse_version_value(value: &str) -> Result<GameVersion, String> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err("gameversion must contain exactly four numeric components".to_string());
    }

    let mut numbers = [0_u32; 4];
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.chars().all(|character| character.is_ascii_digit()) {
            return Err("gameversion contains a non-numeric component".to_string());
        }
        numbers[index] = part
            .parse::<u32>()
            .map_err(|_| "gameversion component is outside the supported range".to_string())?;
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

fn fingerprint_sentinels(install_root: &Path) -> Vec<SentinelFingerprint> {
    SENTINELS
        .iter()
        .filter_map(|relative| {
            let path = relative
                .split('/')
                .fold(install_root.to_path_buf(), |current, part| current.join(part));
            fingerprint_file(&path)
                .ok()
                .map(|(sha256, size_bytes)| SentinelFingerprint {
                    relative_path: (*relative).to_string(),
                    sha256,
                    size_bytes,
                })
        })
        .collect()
}

fn fingerprint_file(path: &Path) -> io::Result<(String, u64)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total = total.saturating_add(read as u64);
    }

    Ok((format!("{:x}", hasher.finalize()), total))
}

fn inventory_packs(install_root: &Path, observed_at: &str) -> Vec<InstalledPackObservation> {
    let Ok(entries) = fs::read_dir(install_root) else {
        return Vec::new();
    };

    let mut packs = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let Some(pack_kind) = pack_kind_from_code(&name) else {
            continue;
        };

        let normalized = name.to_ascii_uppercase();
        let (local_state, size_bytes, marker_count) = match measure_pack(&entry.path()) {
            Ok((bytes, markers)) if bytes > 0 => (PackLocalState::Installed, Some(bytes), markers),
            Ok((bytes, markers)) => (PackLocalState::Partial, Some(bytes), markers),
            Err(_) => (PackLocalState::Unknown, None, 0),
        };

        packs.push(InstalledPackObservation {
            pack_code: normalized,
            pack_kind,
            local_state,
            size_bytes,
            marker_count,
            observed_at: observed_at.to_string(),
        });
    }

    packs.sort_by(|left, right| left.pack_code.cmp(&right.pack_code));
    packs
}

fn measure_pack(root: &Path) -> io::Result<(u64, u64)> {
    let mut stack = vec![root.to_path_buf()];
    let mut total_bytes = 0_u64;
    let mut marker_count = 0_u64;
    let mut entries_seen = 0_usize;

    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            entries_seen += 1;
            if entries_seen > MAX_PACK_WALK_ENTRIES {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "pack inventory exceeded bounded entry limit",
                ));
            }

            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                stack.push(entry.path());
                continue;
            }

            if file_type.is_file() {
                let metadata = entry.metadata()?;
                total_bytes = total_bytes.saturating_add(metadata.len());
                marker_count = marker_count.saturating_add(1);
            }
        }
    }

    Ok((total_bytes, marker_count))
}

fn pack_kind_from_code(name: &str) -> Option<PackKind> {
    let upper = name.to_ascii_uppercase();

    let (prefix, digits) = if upper.starts_with("KIT") {
        ("KIT", &upper[3..])
    } else if upper.len() >= 3 {
        (&upper[..2], &upper[2..])
    } else {
        return None;
    };

    if digits.len() < 2 || digits.len() > 3 || !digits.chars().all(|value| value.is_ascii_digit()) {
        return None;
    }

    match prefix {
        "EP" => Some(PackKind::Expansion),
        "GP" => Some(PackKind::Game),
        "SP" => Some(PackKind::StuffOrKit),
        "FP" => Some(PackKind::Free),
        "KIT" => Some(PackKind::Kit),
        _ => None,
    }
}

fn is_game_install_root(root: &Path) -> bool {
    let bin = root.join("Game").join("Bin");
    let has_binary = bin.join("TS4_x64.exe").is_file();
    let has_default_ini = bin.join("Default.ini").is_file();
    let has_client_data = root.join("Data").join("Client").is_dir();

    (has_binary || has_default_ini) && has_client_data
}

fn infer_provider_from_path(path: &Path) -> GameProvider {
    let value = path.to_string_lossy().to_ascii_lowercase();
    if value.contains("steamapps") {
        GameProvider::Steam
    } else if value.contains("ea games") || value.contains("origin games") {
        GameProvider::EaApp
    } else {
        GameProvider::Unknown
    }
}

fn default_ea_candidates() -> Vec<ProviderCandidate> {
    let mut roots = BTreeSet::new();

    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = env::var_os(variable) {
            let root = PathBuf::from(root);
            roots.insert(root.join("EA Games").join("The Sims 4"));
            roots.insert(root.join("Origin Games").join("The Sims 4"));
        }
    }

    roots
        .into_iter()
        .filter(|root| root.is_dir())
        .map(|root| ProviderCandidate {
            root,
            provider: GameProvider::EaApp,
            evidence: ProviderEvidence::DefaultPath,
        })
        .collect()
}

fn steam_roots() -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();

    if let Some(root) = env::var_os("STEAM_DIR") {
        roots.insert(PathBuf::from(root));
    }

    for variable in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(root) = env::var_os(variable) {
            roots.insert(PathBuf::from(root).join("Steam"));
        }
    }

    roots.into_iter().filter(|root| root.is_dir()).collect()
}

fn discover_steam_candidates(steam_roots: Vec<PathBuf>) -> Vec<ProviderCandidate> {
    let mut library_roots = BTreeSet::new();

    for steam_root in steam_roots {
        library_roots.insert(steam_root.clone());
        let library_file = steam_root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(content) = fs::read_to_string(library_file) {
            for path in parse_steam_library_paths(&content) {
                library_roots.insert(path);
            }
        }
    }

    let mut result = Vec::new();
    for library_root in library_roots {
        let steamapps = library_root.join("steamapps");
        let manifest = steamapps.join(format!("appmanifest_{STEAM_APP_ID}.acf"));
        let Ok(content) = fs::read_to_string(manifest) else {
            continue;
        };
        let Some(install_dir) = parse_steam_install_dir(&content) else {
            continue;
        };

        let root = steamapps.join("common").join(install_dir);
        if root.is_dir() {
            result.push(ProviderCandidate {
                root,
                provider: GameProvider::Steam,
                evidence: ProviderEvidence::SteamManifest,
            });
        }
    }

    result
}

fn parse_steam_library_paths(content: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if !line.starts_with(""path"") {
            continue;
        }
        let Some(value) = quoted_value_after_key(line) else {
            continue;
        };
        paths.push(PathBuf::from(value.replace("\\", "\")));
    }
    paths
}

fn parse_steam_install_dir(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let line = line.trim();
        if !line.starts_with(""installdir"") {
            return None;
        }
        quoted_value_after_key(line)
    })
}

fn quoted_value_after_key(line: &str) -> Option<String> {
    let mut pieces = line.split('"');
    pieces.next()?;
    pieces.next()?;
    let remainder = pieces.next()?;
    let value = pieces.next()?;

    if remainder.trim().is_empty() {
        Some(value.to_string())
    } else {
        None
    }
}

#[cfg(windows)]
fn ea_registry_candidates() -> Vec<ProviderCandidate> {
    use winreg::{
        enums::{KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY},
        HKCU, HKLM,
    };

    let mut roots = BTreeSet::new();
    let paths = [
        r"SOFTWARE\Maxis\The Sims 4",
        r"SOFTWARE\WOW6432Node\Maxis\The Sims 4",
    ];

    for hive in [&HKLM, &HKCU] {
        for key_path in paths {
            for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
                if let Ok(key) = hive.open_subkey_with_flags(key_path, KEY_READ | view) {
                    if let Ok(value) = key.get_value::<String, _>("Install Dir") {
                        let root = PathBuf::from(value.trim_matches('"').trim());
                        if root.is_dir() {
                            roots.insert(root);
                        }
                    }
                }
            }
        }
    }

    roots
        .into_iter()
        .map(|root| ProviderCandidate {
            root,
            provider: GameProvider::EaApp,
            evidence: ProviderEvidence::Registry,
        })
        .collect()
}

#[cfg(not(windows))]
fn ea_registry_candidates() -> Vec<ProviderCandidate> {
    Vec::new()
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
    use tempfile::TempDir;

    fn create_game_root() -> TempDir {
        let temp = TempDir::new().expect("create temp root");
        fs::create_dir_all(temp.path().join("Game").join("Bin")).expect("create bin");
        fs::create_dir_all(temp.path().join("Data").join("Client")).expect("create client");
        fs::write(temp.path().join("Game").join("Bin").join("TS4_x64.exe"), b"fixture")
            .expect("write exe fixture");
        temp
    }

    #[test]
    fn parses_default_ini_version_without_guessing() {
        let version = parse_default_ini_game_version(
            "[Version]\ngameversion = 1.128.90.1030\ncodeversion = 1.0\n",
        )
        .expect("version");

        assert_eq!(version.normalized, "1.128.90.1030");
    }

    #[test]
    fn malformed_default_ini_returns_error() {
        let error = parse_default_ini_game_version("[Version]\ngameversion = 1.bad.90.1030")
            .expect_err("malformed version");
        assert!(error.contains("non-numeric"));
    }

    #[test]
    fn pack_codes_are_classified_conservatively() {
        assert_eq!(pack_kind_from_code("EP01"), Some(PackKind::Expansion));
        assert_eq!(pack_kind_from_code("GP12"), Some(PackKind::Game));
        assert_eq!(pack_kind_from_code("SP81"), Some(PackKind::StuffOrKit));
        assert_eq!(pack_kind_from_code("FP01"), Some(PackKind::Free));
        assert_eq!(pack_kind_from_code("KIT001"), Some(PackKind::Kit));
        assert_eq!(pack_kind_from_code("Mods"), None);
        assert_eq!(pack_kind_from_code("EP1"), None);
    }

    #[test]
    fn empty_pack_is_partial_and_nonempty_pack_is_installed() {
        let game = create_game_root();
        fs::create_dir_all(game.path().join("EP01")).expect("empty pack");
        fs::create_dir_all(game.path().join("GP01")).expect("pack");
        fs::write(game.path().join("GP01").join("ClientFullBuild0.package"), b"pack")
            .expect("pack payload");

        let packs = inventory_packs(game.path(), "test");
        let ep = packs.iter().find(|pack| pack.pack_code == "EP01").expect("ep");
        let gp = packs.iter().find(|pack| pack.pack_code == "GP01").expect("gp");

        assert_eq!(ep.local_state, PackLocalState::Partial);
        assert_eq!(ep.size_bytes, Some(0));
        assert_eq!(gp.local_state, PackLocalState::Installed);
        assert_eq!(gp.size_bytes, Some(4));
    }

    #[test]
    fn sentinel_fallback_records_hashes_without_inventing_version() {
        let game = create_game_root();
        let evidence = LocalVersionProbe.probe(game.path());

        assert!(evidence.version.is_none());
        assert_eq!(evidence.evidence_kind, VersionEvidenceKind::SentinelFingerprint);
        assert_eq!(evidence.confidence, EvidenceConfidence::Unknown);
        assert!(!evidence.sentinels.is_empty());
    }

    #[test]
    fn steam_manifest_discovers_fixture_installation() {
        let temp = TempDir::new().expect("steam temp");
        let library = temp.path().join("library");
        let steamapps = library.join("steamapps");
        let install = steamapps.join("common").join("The Sims 4");
        fs::create_dir_all(install.join("Game").join("Bin")).expect("bin");
        fs::create_dir_all(install.join("Data").join("Client")).expect("client");
        fs::write(install.join("Game").join("Bin").join("TS4_x64.exe"), b"fixture")
            .expect("exe");

        fs::create_dir_all(&steamapps).expect("steamapps");
        fs::write(
            steamapps.join("appmanifest_1222670.acf"),
            "\"AppState\"\n{\n  \"appid\" \"1222670\"\n  \"installdir\" \"The Sims 4\"\n}\n",
        )
        .expect("manifest");

        let candidates = discover_steam_candidates(vec![library]);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].provider, GameProvider::Steam);
        assert_eq!(candidates[0].root, install);
    }

    #[test]
    fn steam_library_paths_parse_escaped_windows_paths() {
        let paths = parse_steam_library_paths(
            "\"libraryfolders\"\n{\n \"1\" { \"path\" \"D:\\\\SteamLibrary\" }\n \"path\" \"E:\\\\Games\"\n}\n",
        );

        assert!(paths.contains(&PathBuf::from(r"E:\Games")));
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
            inspected.build.version.as_ref().map(|version| version.normalized.as_str()),
            Some("1.128.90.1030")
        );
    }
}
