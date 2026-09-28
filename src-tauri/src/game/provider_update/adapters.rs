use std::{collections::BTreeSet, env, fmt, path::PathBuf, process::Command};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderUpdateCapability {
    pub(crate) provider: String,
    pub(crate) supported: bool,
    pub(crate) action_label: String,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderLaunch {
    pub(crate) executable_name: String,
}

#[derive(Debug)]
pub(crate) enum ProviderLaunchError {
    Unsupported(String),
    NotFound(String),
    Io(std::io::Error),
}

impl fmt::Display for ProviderLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(detail) | Self::NotFound(detail) => formatter.write_str(detail),
            Self::Io(error) => write!(formatter, "unable to open update provider: {error}"),
        }
    }
}

impl std::error::Error for ProviderLaunchError {}

impl From<std::io::Error> for ProviderLaunchError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

trait PlatformUpdateAdapter {
    fn provider(&self) -> &'static str;
    fn action_label(&self) -> &'static str;
    fn manual_detail(&self) -> &'static str;
    fn resolve_executable(&self) -> Option<PathBuf>;

    fn capability(&self) -> ProviderUpdateCapability {
        let supported = self.resolve_executable().is_some();
        ProviderUpdateCapability {
            provider: self.provider().to_string(),
            supported,
            action_label: self.action_label().to_string(),
            detail: if supported {
                format!(
                    "{} is available. Sims Mod Health will open the official client and wait for local verification.",
                    self.provider()
                )
            } else {
                self.manual_detail().to_string()
            },
        }
    }

    fn open(&self) -> Result<ProviderLaunch, ProviderLaunchError> {
        let executable = self
            .resolve_executable()
            .ok_or_else(|| ProviderLaunchError::NotFound(self.manual_detail().to_string()))?;
        let executable_name = executable
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                ProviderLaunchError::NotFound(
                    "The provider executable path could not be validated.".to_string(),
                )
            })?
            .to_string();

        Command::new(&executable).spawn()?;

        Ok(ProviderLaunch { executable_name })
    }
}

struct EaAppAdapter;
struct SteamAdapter;
struct UnknownAdapter;

impl PlatformUpdateAdapter for EaAppAdapter {
    fn provider(&self) -> &'static str {
        "ea_app"
    }

    fn action_label(&self) -> &'static str {
        "Open EA app to update"
    }

    fn manual_detail(&self) -> &'static str {
        "EA app was detected for the game, but EADesktop.exe could not be located. Open EA app manually, update The Sims 4, then return to verify."
    }

    fn resolve_executable(&self) -> Option<PathBuf> {
        resolve_ea_executable()
    }
}

impl PlatformUpdateAdapter for SteamAdapter {
    fn provider(&self) -> &'static str {
        "steam"
    }

    fn action_label(&self) -> &'static str {
        "Open Steam to update"
    }

    fn manual_detail(&self) -> &'static str {
        "Steam was detected for the game, but steam.exe could not be located. Open Steam manually, update The Sims 4, then return to verify."
    }

    fn resolve_executable(&self) -> Option<PathBuf> {
        resolve_steam_executable()
    }
}

impl PlatformUpdateAdapter for UnknownAdapter {
    fn provider(&self) -> &'static str {
        "unknown"
    }

    fn action_label(&self) -> &'static str {
        "Update in your game provider"
    }

    fn manual_detail(&self) -> &'static str {
        "The update provider could not be identified. Update The Sims 4 in the client you normally use, then return to verify."
    }

    fn resolve_executable(&self) -> Option<PathBuf> {
        None
    }

    fn open(&self) -> Result<ProviderLaunch, ProviderLaunchError> {
        Err(ProviderLaunchError::Unsupported(
            self.manual_detail().to_string(),
        ))
    }
}

fn adapter(provider: &str) -> Box<dyn PlatformUpdateAdapter> {
    match provider {
        "ea_app" => Box::new(EaAppAdapter),
        "steam" => Box::new(SteamAdapter),
        _ => Box::new(UnknownAdapter),
    }
}

pub(crate) fn capability(provider: &str) -> ProviderUpdateCapability {
    adapter(provider).capability()
}

pub(crate) fn open_provider(provider: &str) -> Result<ProviderLaunch, ProviderLaunchError> {
    adapter(provider).open()
}

fn validated_executable(path: PathBuf, expected_name: &str) -> Option<PathBuf> {
    if !path.is_file()
        || !path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(expected_name))
    {
        return None;
    }

    let canonical = path.canonicalize().ok()?;
    canonical
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(expected_name))
        .then_some(canonical)
}

fn first_valid(
    candidates: impl IntoIterator<Item = PathBuf>,
    expected_name: &str,
) -> Option<PathBuf> {
    candidates
        .into_iter()
        .find_map(|candidate| validated_executable(candidate, expected_name))
}

fn ea_default_candidates() -> Vec<PathBuf> {
    let mut candidates = BTreeSet::new();
    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = env::var_os(variable) {
            let root = PathBuf::from(root)
                .join("Electronic Arts")
                .join("EA Desktop");
            candidates.insert(root.join("EA Desktop").join("EADesktop.exe"));
            candidates.insert(root.join("EADesktop.exe"));
        }
    }
    candidates.into_iter().collect()
}

fn steam_default_candidates() -> Vec<PathBuf> {
    let mut candidates = BTreeSet::new();
    for variable in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(root) = env::var_os(variable) {
            candidates.insert(PathBuf::from(root).join("Steam").join("steam.exe"));
        }
    }
    candidates.into_iter().collect()
}

#[cfg(windows)]
fn resolve_ea_executable() -> Option<PathBuf> {
    let mut candidates = ea_registry_candidates();
    candidates.extend(ea_default_candidates());
    first_valid(candidates, "EADesktop.exe")
}

#[cfg(not(windows))]
fn resolve_ea_executable() -> Option<PathBuf> {
    None
}

#[cfg(windows)]
fn resolve_steam_executable() -> Option<PathBuf> {
    let mut candidates = steam_registry_candidates();
    candidates.extend(steam_default_candidates());
    first_valid(candidates, "steam.exe")
}

#[cfg(not(windows))]
fn resolve_steam_executable() -> Option<PathBuf> {
    None
}

#[cfg(windows)]
fn ea_registry_candidates() -> Vec<PathBuf> {
    use winreg::{
        enums::{KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY},
        HKCU, HKLM,
    };

    let mut candidates = BTreeSet::new();
    let key_paths = [
        r"SOFTWARE\Electronic Arts\EA Desktop",
        r"SOFTWARE\WOW6432Node\Electronic Arts\EA Desktop",
    ];

    for hive in [&HKLM, &HKCU] {
        for key_path in key_paths {
            for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
                let Ok(key) = hive.open_subkey_with_flags(key_path, KEY_READ | view) else {
                    continue;
                };

                for value_name in ["InstallLocation", "Install Dir"] {
                    let Ok(value) = key.get_value::<String, _>(value_name) else {
                        continue;
                    };
                    let root = PathBuf::from(value.trim_matches('"').trim());
                    candidates.insert(root.join("EADesktop.exe"));
                    candidates.insert(root.join("EA Desktop").join("EADesktop.exe"));
                }
            }
        }
    }

    candidates.into_iter().collect()
}

#[cfg(not(windows))]
fn ea_registry_candidates() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(windows)]
fn steam_registry_candidates() -> Vec<PathBuf> {
    use winreg::{enums::KEY_READ, HKCU};

    let mut candidates = BTreeSet::new();
    let Ok(key) = HKCU.open_subkey_with_flags(r"Software\Valve\Steam", KEY_READ) else {
        return Vec::new();
    };

    if let Ok(value) = key.get_value::<String, _>("SteamExe") {
        candidates.insert(PathBuf::from(
            value.replace('/', std::path::MAIN_SEPARATOR_STR),
        ));
    }
    if let Ok(value) = key.get_value::<String, _>("SteamPath") {
        candidates.insert(
            PathBuf::from(value.replace('/', std::path::MAIN_SEPARATOR_STR)).join("steam.exe"),
        );
    }

    candidates.into_iter().collect()
}

#[cfg(not(windows))]
fn steam_registry_candidates() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn unknown_provider_is_manual_only() {
        let capability = capability("unknown");
        assert!(!capability.supported);
        assert_eq!(capability.provider, "unknown");
    }

    #[test]
    fn executable_validation_rejects_wrong_binary_name() {
        let temp = TempDir::new().expect("temp");
        let wrong = temp.path().join("cmd.exe");
        std::fs::write(&wrong, b"fixture").expect("fixture");

        assert!(validated_executable(wrong, "steam.exe").is_none());
    }

    #[test]
    fn executable_validation_accepts_exact_allowlisted_name() {
        let temp = TempDir::new().expect("temp");
        let candidate = temp.path().join("steam.exe");
        std::fs::write(&candidate, b"fixture").expect("fixture");

        let resolved = validated_executable(candidate, "steam.exe").expect("allowed");
        assert_eq!(
            resolved.file_name().and_then(|value| value.to_str()),
            Some("steam.exe")
        );
    }
}
