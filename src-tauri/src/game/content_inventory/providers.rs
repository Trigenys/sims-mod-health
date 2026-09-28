use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use super::{
    GameProvider, InstallationProviderProbe, ProviderCandidate, ProviderEvidence,
};

const STEAM_APP_ID: &str = "1222670";

pub(super) struct EaAppProbe;
pub(super) struct SteamProbe;

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

pub(super) fn is_game_install_root(root: &Path) -> bool {
    let bin = root.join("Game").join("Bin");
    let has_binary = bin.join("TS4_x64.exe").is_file();
    let has_default_ini = bin.join("Default.ini").is_file();
    let has_client_data = root.join("Data").join("Client").is_dir();

    (has_binary || has_default_ini) && has_client_data
}

pub(super) fn infer_provider_from_path(path: &Path) -> GameProvider {
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
    content
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("\"path\""))
        .filter_map(quoted_value_after_key)
        .map(|value| PathBuf::from(value.replace("\\\\", "\\")))
        .collect()
}

fn parse_steam_install_dir(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let line = line.trim();
        if !line.starts_with("\"installdir\"") {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn steam_manifest_discovers_fixture_installation() {
        let temp = TempDir::new().expect("steam temp");
        let library = temp.path().join("library");
        let steamapps = library.join("steamapps");
        let install = steamapps.join("common").join("The Sims 4");
        fs::create_dir_all(install.join("Game").join("Bin")).expect("bin");
        fs::create_dir_all(install.join("Data").join("Client")).expect("client");
        fs::write(
            install.join("Game").join("Bin").join("TS4_x64.exe"),
            b"fixture",
        )
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
            "\"libraryfolders\"\n{\n \"path\" \"E:\\\\Games\"\n}\n",
        );

        assert_eq!(paths, vec![PathBuf::from(r"E:\Games")]);
    }

    #[test]
    fn ea_like_path_is_classified_as_ea_provider() {
        assert_eq!(
            infer_provider_from_path(Path::new(r"C:\Program Files\EA Games\The Sims 4")),
            GameProvider::EaApp
        );
    }

    #[test]
    fn game_root_requires_program_markers() {
        let temp = TempDir::new().expect("temp");
        assert!(!is_game_install_root(temp.path()));

        fs::create_dir_all(temp.path().join("Game").join("Bin")).expect("bin");
        fs::create_dir_all(temp.path().join("Data").join("Client")).expect("client");
        fs::write(
            temp.path().join("Game").join("Bin").join("Default.ini"),
            "[Version]\ngameversion = 1.128.90.1030",
        )
        .expect("default ini");

        assert!(is_game_install_root(temp.path()));
    }
}
